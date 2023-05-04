extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect, self};
use sdl2::sys::{self, SDL_Point};
use serialport::SerialPort;
use std::time::Duration;

mod font;
use font::font::FONT_BYTES;

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

pub fn get_points(bytes : Vec<u8>) -> Vec<Point>
{
    let mut points: Vec<Point> = Vec::new();
    let mut y = 0;
    for byte in bytes.iter()
    {
        for bit in 0..7
        {
            if byte & (1 << bit) != 0
            {
                points.push(Point::new(7 - bit, y));
            }
        }
        y = y + 1;
    }
    points
}

pub fn main() -> Result<(), String> {
    println!("Start");
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("agon-vdp-sdl", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
    let mut char_x = 0;
    let mut char_y = 0;

    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode, ..} => 
                {
                    let mut ascii = keycode.unwrap() as usize;
                    ascii = ascii - 32;
                    if ascii < FONT_BYTES.len()
                    {
                        println!("Pressed keycode:{}", ascii);
                        let points = 		get_points(FONT_BYTES[8*ascii..8*ascii+8].to_vec());
                        canvas.set_draw_color(Color::RGB(255, 255, 255));
                        canvas.set_viewport(Rect::new(char_x * 8,char_y * 8, 8, 8));
                        canvas.draw_points(&points[..]);
                        char_x = char_x + 1;
                        if char_x > 80
                        {
                            char_x = 0;
                            char_y = char_y + 1;
                        }
                    }
                },
                _ => {}
            }
        }

        //canvas.clear();
        canvas.present();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 30));
        // The rest of the game loop goes here...
        if false
        {
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
    }
    println!("Quit");

    Ok(())
}