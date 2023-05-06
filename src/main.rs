extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect, self};
use sdl2::render::Canvas;
use sdl2::sys::{self, SDL_Point};
use sdl2::video::Window;
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

pub fn render_char(canvas : &mut Canvas<Window>, ascii : u8, x : u16, y : u16)
{
    if ascii >= 32
    {
        let shifted_ascii = ascii - 32;
        if shifted_ascii < (FONT_BYTES.len() / 8) as u8
        {
            let start = 8*shifted_ascii as usize;
            let end = start+8 as usize;
            let points = 		get_points(FONT_BYTES[start..end].to_vec());
            canvas.set_draw_color(Color::RGB(255, 255, 255));
            canvas.set_viewport(Rect::new(x as i32,y as i32, 8, 8));
            canvas.draw_points(&points[..]);
        }
    }
}

struct Cursor {
    position_x: u16,
    position_y: u16,
    screen_width: u16,
    screen_height: u16,
    font_width: u16,
    font_height: u16
}

impl Cursor {
    fn new(screen_width: u16, screen_height: u16, font_width: u16, font_height: u16) -> Cursor {
        Cursor {
            position_x: 0,
            position_y: 0,
            screen_width: screen_width,
            screen_height: screen_height,
            font_width: font_width,
            font_height: font_height
        }
    }

    fn home(&mut self) {
        self.position_x = 0;
    }

    fn down(&mut self) {
        self.position_y += self.font_height;
    }

    fn up(&mut self) {
        self.position_y -= self.font_height;
        if self.position_y < 0 {
          self.position_y = 0;
        }
    }

    fn left(&mut self) {
        self.position_x -= self.font_width;
        if self.position_x < 0 {
            self.position_x = 0;
        }
    }

    fn right(&mut self) {
        self.position_x += self.font_width;
        if self.position_x >= self.screen_width {
          self.home();
          self.down();
        }
    }
}
pub fn main() -> Result<(), String> {

    let screen_width = 512;
    let screen_height = 384;
    let font_width = 8;
    let font_height = 8;
    let scale = 2;

    println!("Start");
    let mut cursor = Cursor::new(screen_width, screen_height, font_width, font_height);
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("agon-vdp-sdl", 512*scale, 384*scale)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;
 
    canvas.set_scale(scale as f32, scale as f32);
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();
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
                                render_char(&mut canvas, ascii.try_into().unwrap(), cursor.position_x, cursor.position_y);
                                cursor.right();
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