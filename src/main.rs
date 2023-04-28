extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use serialport::SerialPort;
use std::time::Duration;

pub fn read_serial(mut port : Box<dyn SerialPort>) -> Option<u8>
{
    let mut serial_buf: Vec<u8> = vec![0; 1];
    let mut read_bytes = 0;
        match port.read(serial_buf.as_mut_slice())
        {
            Ok(n) => return Some(serial_buf[0]),
            Err(_e) => return None,
        }
}

pub fn main() -> Result<(), String> {


    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("rust-sdl2 demo: Video", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        canvas.clear();
        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 30));
        // The rest of the game loop goes here...
        // Serial
        let port = serialport::new("/dev/ttyUSB0", 1152_000)
        .timeout(Duration::from_millis(10))
        .open().expect("Failed to open port");
  
        // let ports = serialport::available_ports().expect("No ports found!");
        // for p in ports {
        //     println!("{}", p.port_name);
        // }

        //println!("Read from serial.");
        match read_serial(port)
        {
            Some(n) => match n
            {
                n if n >= 0x20 && n != 0x7F => println!("{}", n as char),
                0x08 => println!("Cursor Left"),
                _n => println!("Unknown Command {:#02x} received!", n),
            }
            None => (),
        }
    }

    Ok(())
}