extern crate sdl2;

use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

use agon_light_vdp::VDP;

use agon_cpu_emulator::AgonMachine;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::sys::KeyCode;

pub fn main() -> Result<(), String> {
    let (tx_VDP2EZ80, rx_VDP2EZ80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_EZ802VDP, rx_EZ802VDP): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let vsync_counter_vdp = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let vsync_counter_ez80 = vsync_counter_vdp.clone();

    

    println!("Start");

    let _cpu_thread = thread::spawn(move || {
        // Prepare the device
        let mut machine = AgonMachine::new(tx_EZ802VDP, rx_VDP2EZ80, vsync_counter_ez80);
        machine.start();
        println!("Cpu thread finished.");
    });

    let scale = 2.0f32;

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("agon-light-emulator", 512, 384)
        .position_centered()
        .resizable()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    //canvas.set_scale(scale, scale);

    let texture_creator = canvas.texture_creator();

    let mut vdp = VDP::VDP::new(canvas, &texture_creator, tx_VDP2EZ80, rx_EZ802VDP, vsync_counter_vdp)?;
    vdp.start();

    let mut event_pump = sdl_context.event_pump()?;
    
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::TextInput { timestamp, window_id, text } => {
                    println!("timestamp {} window_id {}, text {}", timestamp, window_id, text);
                    let ascii = *text.as_bytes().first().unwrap();
                    if ascii < 128 {
                        vdp.send_key(ascii, true);
                        vdp.send_key(ascii, false);
                    } else {
                        println!("Ignored key with ascii > 127: {}", ascii);
                    }
                },
                Event::KeyUp {keycode, keymod, scancode, ..}  | Event::KeyDown {keycode, keymod, scancode, ..} => {
                    match scancode {
                        Some(scancode) => {
                            match scancode {
                                _ => {
                                    let ascii = VDP::VDP::sdl_scancode_to_mos_keycode(scancode, keymod);
                                    if ascii > 0 {
                                        let down = matches!(event, Event::KeyDown{..});
                                        println!("Pressed key:{:?} with mod:{} ascii:{} scancode:{} down:{}", Some(keycode), keymod, ascii, scancode, down);
                                        vdp.send_key(ascii, down);
                                    }
                                },
                            }
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
