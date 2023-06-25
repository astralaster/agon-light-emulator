extern crate sdl2;

use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

use agon_light_vdp::VDP;

use agon_cpu_emulator::{ AgonMachine, AgonMachineConfig };
use sdl2::event::Event;

use clap::Parser;
use sdl2::pixels::{PixelFormatEnum};
use sdl2::surface::Surface;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Scaling factor of the ouput window
    #[arg(short, long, default_value_t = 2)]
    scale: u8,
}

fn main() -> Result<(), String> {
    let args = Args::parse();

    let (tx_vdp_to_ez80, rx_vdp_to_ez80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_ez80_to_vdp, rx_ez80_to_vdp): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let vsync_counter_vdp = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let vsync_counter_ez80 = vsync_counter_vdp.clone();

    

    println!("Start");

    let _cpu_thread = thread::spawn(move || {
        // Prepare the device
        let mut machine = AgonMachine::new(AgonMachineConfig {
            to_vdp: tx_ez80_to_vdp,
            from_vdp: rx_vdp_to_ez80,
            vsync_counter: vsync_counter_ez80,
            clockspeed_hz: 18_432_000
        });
        //machine.set_sdcard_directory(std::env::current_dir().unwrap().join("sdcard"));
        machine.start(None);
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
    //canvas.set_scale(scale, scale);

    let texture_creator = canvas.texture_creator();

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
                                    println!("Pressed key: scancode:{:?} with mod:{} down:{}", scancode, keymod, down);
                                    vdp.send_key(scancode, keymod, down);
                                },
                        None => println!("Key without scancode pressed."),
                    }
                },
                _ => (),
            }
        }

        vdp.run();
    }

    println!("Quit");

    Ok(())
}
