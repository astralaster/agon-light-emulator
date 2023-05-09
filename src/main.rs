extern crate sdl2;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect, self};
use sdl2::render::Canvas;
use sdl2::sys::{self, SDL_Point};
use sdl2::video::Window;
use serialport::SerialPort;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

mod font;
use font::font::FONT_BYTES;

use iz80::*;
use std::io::Write;

const ROM_SIZE: usize = 0x40000; // 256 KiB
const RAM_SIZE: usize = 0x80000; // 512 KiB
const MEM_SIZE: usize = ROM_SIZE + RAM_SIZE;





pub struct AgonMachine {
    mem: [u8; MEM_SIZE],
    io: [u8; 65536],
    tx: Sender<u8>,
    rx: Receiver<u8>
}

impl AgonMachine {
    /// Returns a new AgonMachine instance
    pub fn new(tx : Sender<u8>, rx : Receiver<u8>) -> AgonMachine {
        AgonMachine {
            mem: [0; MEM_SIZE],
            io: [0; 65536],
            tx: tx,
            rx: rx
        }
    }
}

// impl Default for AgonMachine {
//     fn default() -> Self {
//         Self::new()
//     }
// }

impl Machine for AgonMachine {
    fn peek(&self, address: u32) -> u8 {
        self.mem[address as usize]
    }
    fn poke(&mut self, address: u32, value: u8) {
        self.mem[address as usize] = value;
    }

    fn port_in(&mut self, address: u16) -> u8 {
        //println!("IN({:02X}) = 0", address);
        if address == 0xa2 {
            0x0 // UART0 clear to send
        } else if address == 0xc5 {
            0x40
            // UART_LSR_ETX		EQU 	%40
        } else if address == 0x81 /* timer0 low byte */ {
            0x0
        } else if address == 0x82 /* timer0 high byte */ {
            0x0
        } else {
            self.io[address as usize]
        }
    }
    fn port_out(&mut self, address: u16, value: u8) {
        if address == 0xc0 /* UART0_REG_THR */ {
            /* Echo data from VDP to stdout */
            self.tx.send(value);
            //print!("{}", char::from_u32(value as u32).unwrap());
            //std::io::stdout().flush().unwrap();
        }
        self.io[address as usize] = value;
    }
}

pub fn read_serial(port : &mut Box<dyn SerialPort>) -> Option<u8>
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

fn cls(canvas : &mut Canvas<Window>, cursor : &mut Cursor) {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    cursor.position_x = 0;
    cursor.position_y = 0;
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
    let serial_active = false;
    let mut esp_boot_output = true;

    let (tx_VDP2EZ80, rx_VDP2EZ80): (Sender<u8>, Receiver<u8>) = mpsc::channel();
    let (tx_EZ802VDP, rx_EZ802VDP): (Sender<u8>, Receiver<u8>) = mpsc::channel();

    println!("Start");

    let cpu_thread = thread::spawn(move || {
        // Prepare the device
        let mut machine = AgonMachine::new(tx_EZ802VDP, rx_VDP2EZ80);
        let mut cpu = Cpu::new_ez80();
        //cpu.set_trace(true);

        // Load program inline or from a file with:
        let code = match std::fs::read("MOS.bin") {
            Ok(data) => data,
            Err(e) => {
                println!("Error opening MOS.bin: {:?}", e);
                std::process::exit(-1);
            }
        };

        for (i, e) in code.iter().enumerate() {
            machine.poke(i as u32, *e);
        }

        // Run emulation
        cpu.state.set_pc(0x0000);

        loop {
            cpu.execute_instruction(&mut machine);
        }
    });

    // let mut port = serialport::new("/dev/ttyUSB0", 115200)
    //     .timeout(Duration::from_millis(10))
    //     .open().expect("Failed to open serial port.");

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
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 100));
        // The rest of the game loop goes here...
        if serial_active
        {
            // // Serial

    
            // // let ports = serialport::available_ports().expect("No ports found!");
            // // for p in ports {
            // //     println!("{}", p.port_name);
            // // }
    
            // //println!("Read from serial.");
            // match read_serial(&mut port)
            // {
            //     Some(n) => match n
            //     {
            //         n if n >= 0x20 && n != 0x7F => {
            //             println!("Received character: {}", n as char);
            //             render_char(&mut canvas, n, cursor.position_x, cursor.position_y);
            //             cursor.right();  
            //         },
            //         0x08 => {println!("Cursor left."); cursor.left();},
            //         0x09 => {println!("Cursor right."); cursor.right();},
            //         0x0A => {println!("Cursor down."); cursor.down();},
            //         0x0B => {println!("Cursor up."); cursor.up();},
            //         0x0C => {
            //             println!("CLS.");
            //             cls(&mut canvas, &mut cursor);
            //         },
            //         0x0D => {println!("Cursor home."); cursor.home();},
            //         0x0E => {println!("PageMode ON?");},
            //         0x0F => {println!("PageMode OFF?");},
            //         0x10 => {println!("CLG?");},
            //         0x11 => {println!("COLOUR?");},
            //         0x12 => {println!("GCOL?");},
            //         0x13 => {println!("Define Logical Colour?");},
            //         0x16 => {println!("MODE?");},
            //         0x17 => {
            //             println!("VDU23.");
            //             if esp_boot_output {
            //                 println!("ESP output ends here. Now CLS.");
            //                 cls(&mut canvas, &mut cursor);
            //                 esp_boot_output = false;
            //             }
            //             else {
            //                 match read_serial(&mut port) {
            //                     Some(n) => match n {
            //                         0x00 => {
            //                             println!("Video System Control.");
            //                             match read_serial(&mut port) {
            //                                 Some(n) => match n {
            //                                     0x80 => println!("VDP_GP"),
            //                                     0x81 => println!("VDP_KEYCODE"),
            //                                     _ => println!("Unknown VSC command: {:#02X?}.", n),
            //                                 },
            //                                 None => (),
            //                             }
            //                         },
            //                         0x01 => println!("Cursor Control?"),
            //                         0x07 => println!("Scroll?"),
            //                         0x1B => println!("Sprite Control?"),
            //                         _ => println!("Unknown VDU command: {:#02X?}.", n),
            //                     },
            //                     None => (),
            //                 }
            //             }
            //         },
            //         0x19 => {println!("PLOT?");},
            //         0x1D => {println!("VDU_29?");},
            //         0x1E => {println!("Home."); cursor.home();},
            //         0x1F => {println!("TAB?");},
            //         0x7F => {println!("BACKSPACE?");},
            //         _n => println!("Unknown Command {:#02X?} received!", n),
            //     }
            //     None => (),
            // }
        }
        else {

            match rx_EZ802VDP.recv().unwrap() {
                n if n >= 0x20 && n != 0x7F => {
                    println!("Received character: {}", n as char);
                    render_char(&mut canvas, n, cursor.position_x, cursor.position_y);
                    cursor.right();  
                },
                0x08 => {println!("Cursor left."); cursor.left();},
                0x09 => {println!("Cursor right."); cursor.right();},
                0x0A => {println!("Cursor down."); cursor.down();},
                0x0B => {println!("Cursor up."); cursor.up();},
                0x0C => {
                    println!("CLS.");
                    cls(&mut canvas, &mut cursor);
                },
                0x0D => {println!("Cursor home."); cursor.home();},
                0x0E => {println!("PageMode ON?");},
                0x0F => {println!("PageMode OFF?");},
                0x10 => {println!("CLG?");},
                0x11 => {println!("COLOUR?");},
                0x12 => {println!("GCOL?");},
                0x13 => {println!("Define Logical Colour?");},
                0x16 => {println!("MODE?");},
                0x17 => {
                    println!("VDU23.");
                    if esp_boot_output {
                        println!("ESP output ends here. Now CLS.");
                        cls(&mut canvas, &mut cursor);
                        esp_boot_output = false;
                    }
                    else {
                        match rx_EZ802VDP.recv().unwrap() {
                            0x00 => {
                                println!("Video System Control.");
                                match rx_EZ802VDP.recv().unwrap() {
                                    0x80 => println!("VDP_GP"),
                                    0x81 => println!("VDP_KEYCODE"),
                                    n => println!("Unknown VSC command: {:#02X?}.", n),
                                }
                            },
                            0x01 => println!("Cursor Control?"),
                            0x07 => println!("Scroll?"),
                            0x1B => println!("Sprite Control?"),
                            n => println!("Unknown VDU command: {:#02X?}.", n),
                        }
                    }
                },
                0x19 => {println!("PLOT?");},
                0x1D => {println!("VDU_29?");},
                0x1E => {println!("Home."); cursor.home();},
                0x1F => {println!("TAB?");},
                0x7F => {println!("BACKSPACE?");},
                n => println!("Unknown Command {:#02X?} received!", n),
            }
        }
    }
    println!("Quit");

    Ok(())
}