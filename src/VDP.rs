
use core::panic;
use std::sync::mpsc::{Sender, Receiver};
use std::time::{Instant, Duration};

use sdl2::Sdl;
use sdl2::event::Event;
use sdl2::keyboard::{self, Mod, Scancode};
use sdl2::keyboard::Keycode;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, SurfaceCanvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};
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

struct VideoMode{
    colors: u8,
    screen_width: u32,
    screen_height: u32,
    refresh_rate: u8,
}

static VIDEO_MODES: [VideoMode; 4] = [VideoMode{colors: 2, screen_width: 1024, screen_height: 768, refresh_rate: 60},
                                    VideoMode{colors: 16, screen_width: 512, screen_height: 384, refresh_rate: 60},
                                    VideoMode{colors: 64, screen_width: 320, screen_height: 240, refresh_rate: 75},
                                    VideoMode{colors: 16, screen_width: 640, screen_height: 480, refresh_rate: 60}];

pub struct VDP<'a> {
    cursor: Cursor,
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    texture_creator: &'a TextureCreator<WindowContext>,
    tx: Sender<u8>,
    rx: Receiver<u8>,
    foreground_color: sdl2::pixels::Color,
    background_color: sdl2::pixels::Color,
    test_color: sdl2::pixels::Color,
    cursor_active: bool,
    cursor_last_change: Instant,
    vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
    last_vsync: Instant,
    current_video_mode: &'static VideoMode,
}

impl VDP<'_> {
    pub fn new(canvas: Canvas<Window>, texture_creator: &TextureCreator<WindowContext>, tx: Sender<u8>, rx: Receiver<u8>, vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>) -> Result<VDP, String> {
        let mode =  &VIDEO_MODES[1];

        let texture = texture_creator.create_texture(None, sdl2::render::TextureAccess::Target, mode.screen_width, mode.screen_height).unwrap();
     
        Ok(VDP {
            cursor: Cursor::new(mode.screen_width as i32, mode.screen_height as i32, 8, 8),
            canvas: canvas,
            texture: texture,
            texture_creator: texture_creator,
            tx: tx,
            rx: rx,
            foreground_color: Color::RGB(255, 255, 255),
            background_color: Color::RGB(0, 0, 0),
            test_color: Color::RGB(255, 0, 0),
            cursor_active: false,
            cursor_last_change: Instant::now(),
            vsync_counter: vsync_counter,
            last_vsync: Instant::now(),
            current_video_mode: mode,
        })
    }

    pub fn start(&mut self) {
        self.change_mode(1);
        self.bootscreen();
    }

    pub fn run(&mut self) {
        self.do_comms();
        
        
        if self.last_vsync.elapsed().as_micros() >  (1_000_000u32 / self.current_video_mode.refresh_rate as u32).into() {
            self.vsync_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.last_vsync = Instant::now();


            let result = self.canvas.copy(&self.texture, None, None);
            if result.is_err() {
                panic!("Fail!");
            }
            self.blink_cusor();
            self.canvas.present();
        }
    }

    fn change_mode(&mut self, mode: usize) {
        self.current_video_mode = &VIDEO_MODES[mode];
        self.cursor.screen_height = self.current_video_mode.screen_height as i32;
        self.cursor.screen_width = self.current_video_mode.screen_width as i32;
        self.canvas.window_mut().set_size(self.current_video_mode.screen_width, self.current_video_mode.screen_height);
        self.texture = self.texture_creator.create_texture(None, sdl2::render::TextureAccess::Target, self.current_video_mode.screen_width, self.current_video_mode.screen_height).unwrap();
        self.cls();
    }

    fn get_points_from_font(bytes : Vec<u8>) -> Vec<Point>
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
    
    fn render_char(&mut self, ascii: u8)
    {
        //println!("Render {:#02X?}", ascii);
        if ascii >= 32 {
            let shifted_ascii = ascii - 32;
            let start = (8 * shifted_ascii as u32) as usize;
            let end = start+8 as usize;
            let mut points = Self::get_points_from_font(FONT_BYTES[start..end].to_vec());
            
            for point in points.iter_mut() {
                point.x += self.cursor.position_x;
                point.y += self.cursor.position_y;
            }

            self.canvas.with_texture_canvas(&mut self.texture, |texture_canvas| {
                texture_canvas.set_draw_color(self.background_color);
                texture_canvas.fill_rect(Rect::new(self.cursor.position_x, self.cursor.position_y, 8, 8));
                texture_canvas.set_draw_color(self.foreground_color);
                texture_canvas.draw_points(&points[..]);
            });
        }
    }

    fn bootscreen(&mut self) {
        let boot_message = "Agon Quark VDP Version 1.03";
        for byte in boot_message.as_bytes() {
            self.render_char(*byte);
            self.cursor.right();
        }
        self.cursor.down();
        self.cursor.home();
    }

    fn blink_cusor(&mut self) {
        if self.cursor_last_change.elapsed().as_millis() > 500 {
            self.cursor_active = !self.cursor_active;
            self.cursor_last_change = Instant::now();
        }
        if self.cursor_active {
            self.canvas.set_draw_color(self.foreground_color);
        } else {
            self.canvas.set_draw_color(self.background_color);
        }

        let output_size = self.canvas.output_size().unwrap();
        let scale_x = output_size.0 as f32 / self.current_video_mode.screen_width as f32;
        let scale_y = output_size.1 as f32 / self.current_video_mode.screen_height as f32;

        self.canvas.fill_rect(Rect::new((self.cursor.position_x as f32 * scale_x) as i32, (self.cursor.position_y as f32 * scale_y) as i32, 8u32 * scale_x as u32, 8u32 * scale_y as u32));
    }


    fn backspace(&mut self) {
        self.cursor.left();
        self.render_char(b' ');
    }

    
    pub fn cls(&mut self) {
        self.canvas.with_texture_canvas(&mut self.texture, |texture_canvas| {
            texture_canvas.set_draw_color(self.background_color);
            texture_canvas.clear();
        });
        self.cursor.position_x = 0;
        self.cursor.position_y = 0;
    }

    pub fn send_key(&self, keycode: u8, down: bool){
        let mut keyboard_packet: Vec<u8> = vec![keycode, 0, 0, down as u8];
		self.send_packet(0x1, keyboard_packet.len() as u8, &mut keyboard_packet);
    }

    fn send_cursor_position(&self) {
        let mut cursor_position_packet: Vec<u8> = vec![(self.cursor.position_x / self.cursor.font_width) as u8,
        (self.cursor.position_y / self.cursor.font_height) as u8];
        self.send_packet(0x02, cursor_position_packet.len() as u8, &mut cursor_position_packet);	
    }

    pub fn sdl_scancode_to_mos_keycode(scancode: sdl2::keyboard::Scancode, keymod: sdl2::keyboard::Mod) -> u8{
        match scancode {
            Scancode::Left => 0x08,
            Scancode::Tab => 0x09,
            Scancode::Right => 0x15,
            Scancode::Down => 0x0A,
            Scancode::Backspace => 0x7F,
            Scancode::Return => 0x0D,
            Scancode::Escape => 0x1B,
            _ => 0x00,
        }
    }
    
    fn send_packet(&self, code: u8, len: u8, data: &mut Vec<u8>) {
        let mut output: Vec<u8> = Vec::new();
        output.push(code + 0x80 as u8); 
        output.push(len);
        output.append(data);
        for byte in output.iter() {
            self.tx.send(*byte);
        }
        println!("Send packet to MOS: {:#02X?}", output);
    }
    

    fn do_comms(&mut self) {
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
                    0x0A => {
                        println!("Cursor down.");
                        self.cursor.down();
                        let overdraw = self.cursor.position_y - self.current_video_mode.screen_height as i32 + self.cursor.font_height;
                        if overdraw > 0 {
                            println!("Need to scroll! Overdraw: {}", overdraw);
                            let mut scrolled_texture = self.texture_creator.create_texture(None, sdl2::render::TextureAccess::Target, self.current_video_mode.screen_width, self.current_video_mode.screen_height).unwrap();
                            self.canvas.with_texture_canvas(&mut scrolled_texture, |texture_canvas| {
                                texture_canvas.set_draw_color(self.background_color);
                                texture_canvas.clear();
                                let rect_src = Rect::new(0, overdraw, self.current_video_mode.screen_width, self.current_video_mode.screen_height - overdraw as u32);
                                let rect_dst = Rect::new(0, 0, self.current_video_mode.screen_width, self.current_video_mode.screen_height - overdraw as u32);
                                texture_canvas.copy(&self.texture, rect_src, rect_dst);
                            });
                            self.texture = scrolled_texture;
                            self.cursor.position_y -= overdraw;
                        }
                    },
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
                    0x16 => {
                        println!("MODE.");
                        let mode = self.rx.recv().unwrap();
                        self.change_mode(mode.into());
                        self.send_mode_information();
                    },
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
                                    0x82 => {
                                        println!("Send Cursor Position");
                                        self.send_cursor_position();
                                    },
                                    0x86 => {
                                        println!("Mode Information");
                                        self.send_mode_information();
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
                    0x1F => {println!("TAB?");
                             let x = self.rx.recv().unwrap() as i32 * self.cursor.font_width;
                             let y = self.rx.recv().unwrap() as i32 * self.cursor.font_height;
                             if (x < self.cursor.screen_width &&
                                 y < self.cursor.screen_height)
                             {
                                 self.cursor.position_x = x;
                                 self.cursor.position_y = y;
                             }
                    },
                    0x7F => {
                        println!("BACKSPACE.");
                        self.backspace();
                    },
                    n => println!("Unknown Command {:#02X?} received!", n),
                }
            },
            Err(_e) => ()
        }
    }

    fn send_mode_information(&mut self) {
        println!("Screen width {} Screen height {}", self.cursor.screen_width, self.cursor.screen_height);
        let mut packet: Vec<u8> = vec![
            self.cursor.screen_width.to_le_bytes()[0],
            self.cursor.screen_width.to_le_bytes()[1],
            self.cursor.screen_height.to_le_bytes()[0],
            self.cursor.screen_height.to_le_bytes()[1],
            (self.cursor.screen_width / self.cursor.font_width) as u8,
            (self.cursor.screen_height / self.cursor.font_height) as u8,
            self.current_video_mode.colors,
         ];
        self.send_packet(0x06, packet.len() as u8, &mut packet);
    }
}
