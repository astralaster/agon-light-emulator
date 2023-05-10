extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

mod VDP;

use iz80::AgonMachine;

pub fn main() -> Result<(), String> {

    let screen_width = 512;
    let screen_height = 384;
    let font_width = 8;
    let font_height = 8;
    let scale = 1;
    let serial_active = false;
    let mut esp_boot_output = true;

    let (tx_VDP2EZ80, rx_VDP2EZ80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_EZ802VDP, rx_EZ802VDP): (Sender<u8>, Receiver<u8>) = mpsc::channel();

    println!("Start");

    let cpu_thread = thread::spawn(move || {
        // Prepare the device
        let mut machine = AgonMachine::new(tx_EZ802VDP, rx_VDP2EZ80);
        machine.start();
        println!("Cpu thread finished.");
    });

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("agon-light-emulator", 512*scale, 384*scale)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
 
    canvas.set_scale(scale as f32, scale as f32);
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let mut vdp = VDP::VDP::new(canvas, tx_VDP2EZ80, rx_EZ802VDP);

    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode, keymod, ..} => 
                {
                    match keycode {
                        Some(keycode) =>
                        {
                            let mut ascii = keycode as u8;
                            if ascii < 127 && ascii >= 32
                            {
                                println!("Pressed key:{} with mod:{} ascii:{}", keycode, keymod, ascii);
                                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) || keymod.contains(Mod::CAPSMOD)
                                {
                                    if ascii < 65 {
                                        ascii -= 16;
                                    }
                                    else {
                                        ascii -= 32;
                                    }
                                }
                                vdp.send_key(ascii);
                                // render_char(&mut canvas, ascii.try_into().unwrap(), cursor.position_x, cursor.position_y);
                                // cursor.right();
                            }
                            else
                            {
                                println!("Ignored key:{} with mod:{} ascii:{}", keycode, keymod, ascii);
                            }
                        },
                        None => println!("Invalid key pressed."),
                    }

                },
                _ => {}
            }
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 100));
        vdp.run();
    }
    println!("Quit");

    Ok(())
}