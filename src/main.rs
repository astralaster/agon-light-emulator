extern crate sdl2;

use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

use agon_light_vdp::VDP;

use agon_cpu_emulator::{ AgonMachine, AgonMachineConfig };
use agon_cpu_emulator::debugger::{ DebugCmd, DebugResp, DebuggerConnection };
use sdl2::event::Event;
use log;
use clap::Parser;
use sdl2::pixels::{PixelFormatEnum};
use sdl2::surface::Surface;
mod logger;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Scaling factor of the ouput window
    #[arg(short, long, default_value_t = 2)]
    scale: u8,
    /// Debugger (disables logging)
    #[arg(short, long, default_value_t = false)]
    debugger: bool,
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
    /// Log level (trace, debug, info, warn, error, off)
    #[arg(short, long, default_value_t = String::from("info"))]
    log_level: String,
    /// Path to the emulated sdcard directory
    #[arg(long, default_value_t = String::from("sdcard"))]
    sdcard: String,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    let (tx_vdp_to_ez80, rx_vdp_to_ez80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_ez80_to_vdp, rx_ez80_to_vdp): (Sender<u8>, Receiver<u8>) = mpsc::channel();

    let (tx_cmd_debugger, rx_cmd_debugger): (Sender<DebugCmd>, Receiver<DebugCmd>) = mpsc::channel();
    let (tx_resp_debugger, rx_resp_debugger): (Sender<DebugResp>, Receiver<DebugResp>) = mpsc::channel();

    let vsync_counter_vdp = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let vsync_counter_ez80 = vsync_counter_vdp.clone();

    let debugger_con = if args.debugger {
        let _debugger_thread = thread::spawn(move || {
            agon_light_emulator_debugger::start(tx_cmd_debugger, rx_resp_debugger);
        });
        Some(DebuggerConnection { tx: tx_resp_debugger, rx: rx_cmd_debugger })
    } else {
        None
    };

    let _cpu_thread = thread::spawn(move || {
        // Prepare the device
        let mut machine = AgonMachine::new(AgonMachineConfig {
            to_vdp: tx_ez80_to_vdp,
            from_vdp: rx_vdp_to_ez80,
            vsync_counter: vsync_counter_ez80,
            clockspeed_hz: 18_432_000
        });
        machine.set_sdcard_directory(std::env::current_dir().unwrap().join(args.sdcard));
        machine.start(debugger_con);
        println!("Cpu thread finished.");
    });

    let scale_window = args.scale;

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let audio_subsystem = sdl_context.audio()?;
    
    let window_title = format!("agon-light-emulator ({})", env!("GIT_HASH"));

    let agon_logo_height = 56;
    let agon_logo_width = 56;
    let mut icon_data = include_bytes!("../assets/icon.data").to_vec();
    let window_icon = Surface::from_data(&mut icon_data, agon_logo_width, agon_logo_height, agon_logo_width*4, PixelFormatEnum::ABGR8888).unwrap();

    let mut window = video_subsystem
    .window(window_title.as_str(), 512, 384)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;
    window.set_icon(window_icon);

    let canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let texture_creator = canvas.texture_creator();

    if !args.debugger {
        match args.log_level.as_str() {
            "info" => logger::init(logger::Lvl::Info).unwrap(),
            "debug" => logger::init(logger::Lvl::Debug).unwrap(),
            "warn" => logger::init(logger::Lvl::Warn).unwrap(),
            "error" => logger::init(logger::Lvl::Error).unwrap(),
            "off" => logger::init(logger::Lvl::Off).unwrap(),
            "trace" => logger::init(logger::Lvl::Trace).unwrap(),
            _ => println!("Unknown loglevel: {}", args.log_level)
        }
    }

    let mut vdp = VDP::VDP::new(canvas, &texture_creator, scale_window, tx_vdp_to_ez80, rx_ez80_to_vdp, vsync_counter_vdp, audio_subsystem)?;
    vdp.start();

    let mut event_pump = sdl_context.event_pump()?;
    
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyUp {keymod, scancode, ..}  | Event::KeyDown {keymod, scancode, ..} => {
                    match scancode {
                        Some(scancode) => {
                                    let down = matches!(event, Event::KeyDown{..});
                                    log::debug!("Pressed key: scancode:{} with mod:{} down:{}", scancode, keymod, down);
                                    vdp.send_key(scancode, keymod, down);
                                },
                        None => log::warn!("Key without scancode pressed."),
                    }
                },
                _ => (),
            }
        }

        vdp.run();
    }

    Ok(())
}
