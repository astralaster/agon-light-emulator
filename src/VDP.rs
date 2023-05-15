
use std::sync::mpsc::{Sender, Receiver};
use std::time::Instant;

use sdl2::keyboard::{self, Mod};
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;
mod font;
use font::font::FONT_BYTES;
struct Cursor {
    position_x: i32,
    position_y: i32,
    screen_width: i32,
    screen_height: i32,
    font_width: i32,
    font_height: i32
}

impl Cursor {
    fn new(screen_width: i32 , screen_height: i32, font_width: i32, font_height: i32) -> Cursor {
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

pub struct VDP {
    cursor: Cursor,
    canvas: Canvas<Window>,
    tx: Sender<u8>,
    rx: Receiver<u8>,
    foreground_color: sdl2::pixels::Color,
    background_color: sdl2::pixels::Color,
    test_color: sdl2::pixels::Color,
    vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
    last_vsync: Instant,
}

impl VDP {
    pub fn new(canvas: Canvas<Window>, tx : Sender<u8>, rx : Receiver<u8>, vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>) -> VDP {
        VDP {
            cursor: Cursor::new(canvas.window().drawable_size().0 as i32, canvas.window().drawable_size().1 as i32, 8, 8),
            canvas: canvas,
            tx: tx,
            rx: rx,
            foreground_color: Color::RGB(255, 255, 255),
            background_color: Color::RGB(0, 0, 0),
            test_color: Color::RGB(255, 0, 0),
            vsync_counter: vsync_counter,
            last_vsync: Instant::now(),
        }
    }

    fn get_points(bytes : Vec<u8>) -> Vec<Point>
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
    
    fn render_char(&mut self, ascii : u8)
    {
        if ascii >= 32
        {
            let shifted_ascii = ascii - 32;
            if shifted_ascii < (FONT_BYTES.len() / 8) as u8
            {
                self.canvas.set_draw_color(self.background_color);
                self.canvas.fill_rect(Rect::new(self.cursor.position_x as i32, self.cursor.position_y as i32, 8, 8));
                let start = 8*shifted_ascii as usize;
                let end = start+8 as usize;
                let points = Self::get_points(FONT_BYTES[start..end].to_vec());
                self.canvas.set_draw_color(self.foreground_color);
                let viewport = self.canvas.viewport();
                self.canvas.set_viewport(Rect::new(self.cursor.position_x as i32, self.cursor.position_y as i32, 8, 8));
                self.canvas.draw_points(&points[..]);
                self.canvas.set_viewport(viewport);
                self.canvas.present();
            }
        }
    }

    pub fn backspace(&mut self) {
        self.cursor.left();
        self.render_char(b' ');
    }

    
    pub fn cls(&mut self) {
        self.canvas.set_draw_color(self.background_color);
        self.canvas.clear();
        self.cursor.position_x = 0;
        self.cursor.position_y = 0;
    }

    pub fn send_key(&mut self, keycode: u8, up: bool){
        let mut keyboard_packet: Vec<u8> = vec![keycode, 0, 0, up as u8];
		self.send_packet(0x1, keyboard_packet.len() as u8, &mut keyboard_packet);
    }

    pub fn sdl_keycode_to_mos_keycode(keycode: sdl2::keyboard::Keycode, keymod: sdl2::keyboard::Mod) -> u8{
        match keycode {
            Keycode::Left => 0x08,
            Keycode::Tab => 0x09,
            Keycode::Right => 0x15,
            Keycode::Down => 0x0A,
            Keycode::Backspace => 0x7F,
            _ => {
                let mut ascii = keycode as u8;
                if keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD) || keymod.contains(Mod::CAPSMOD)
                {
                    if ascii < 65 {
                        ascii -= 16;
                    }
                    else {
                        ascii -= 32;
                    }
                }
                ascii
            },
        }
    }


    fn send_packet(&mut self, code: u8, len: u8, data: &mut Vec<u8>) {
        let mut output: Vec<u8> = Vec::new();
        output.push(code + 0x80 as u8); 
        output.push(len);
        output.append(data);
        for byte in output.iter() {
            self.tx.send(*byte);
        }
        println!("Send packet to MOS: {:#02X?}", output);
    }
    

    pub fn run(&mut self) {
        match self.rx.try_recv() {
            Ok(n) => {
                match n {
                    n if n >= 0x20 && n != 0x7F => {
                        println!("Received character: {}", n as char);
                        self.render_char(n);
                        self.cursor.right();  
                    },
                    0x08 => {println!("Cursor left."); self.cursor.left();},
                    0x09 => {println!("Cursor right."); self.cursor.right();},
                    0x0A => {println!("Cursor down."); self.cursor.down();},
                    0x0B => {println!("Cursor up."); self.cursor.up();},
                    0x0C => {
                        println!("CLS.");
                        self.cls();
                    },
                    0x0D => {println!("Cursor home."); self.cursor.home();},
                    0x0E => {println!("PageMode ON?");},
                    0x0F => {println!("PageMode OFF?");},
                    0x10 => {println!("CLG?");},
                    0x11 => {println!("COLOUR?");},
                    0x12 => {println!("GCOL?");},
                    0x13 => {println!("Define Logical Colour?");},
                    0x16 => {println!("MODE?");},
                    0x17 => {
                        println!("VDU23.");
                        match self.rx.recv().unwrap() {
                            0x00 => {
                                println!("Video System Control.");
                                match self.rx.recv().unwrap() {
                                    0x80 => {
                                        println!("VDP_GP.");
                                        let mut packet = Vec::new();
                                        packet.push(self.rx.recv().unwrap());
                                        self.send_packet(0x00, packet.len() as u8, &mut packet);
                                    },
                                    0x81 => println!("VDP_KEYCODE"),
                                    0x86 => {
                                        println!("Mode Information");
                                        println!("Screen width {} Screen height {}", self.cursor.screen_width, self.cursor.screen_height);
                                        let mut packet: Vec<u8> = vec![
                                            self.cursor.screen_width.to_le_bytes()[0],
                                            self.cursor.screen_width.to_le_bytes()[1],
                                            self.cursor.screen_height.to_le_bytes()[0],
                                            self.cursor.screen_height.to_le_bytes()[1],
                                            (self.cursor.screen_width / self.cursor.font_width) as u8,
                                            (self.cursor.screen_height / self.cursor.font_height) as u8,
                                            16
                                         ];
                                        self.send_packet(0x06, packet.len() as u8, &mut packet);
                                    },
                                    n => println!("Unknown VSC command: {:#02X?}.", n),
                                }
                            },
                            0x01 => println!("Cursor Control?"),
                            0x07 => println!("Scroll?"),
                            0x1B => println!("Sprite Control?"),
                            n => println!("Unknown VDU command: {:#02X?}.", n),
                        }
                    },
                    0x19 => {println!("PLOT?");},
                    0x1D => {println!("VDU_29?");},
                    0x1E => {println!("Home."); self.cursor.home();},
                    0x1F => {println!("TAB?");},
                    0x7F => {
                        println!("BACKSPACE.");
                        self.backspace();
                    },
                    n => println!("Unknown Command {:#02X?} received!", n),
                }
            },
            Err(_e) => ()
        }
        // a fake vsync every 16ms
        if self.last_vsync.elapsed().as_millis() > 16 {
            self.vsync_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.last_vsync = Instant::now();
        }
    }
}
