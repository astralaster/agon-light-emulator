
use std::sync::mpsc::{Sender, Receiver};
use std::time::Instant;

use sdl2::Sdl;
use sdl2::event::Event;
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

struct VideoMode{
    colors: u8,
    screen_width: u32,
    screen_height: u32,
    refresh_rate: u8,
}

static VIDEO_MODES: [VideoMode; 4] = [VideoMode{colors: 2, screen_width: 1024, screen_height: 768, refresh_rate: 60},
                                    VideoMode{colors: 16, screen_width: 512, screen_height: 384, refresh_rate: 60},
                                    VideoMode{colors: 64, screen_width: 320, screen_height: 240, refresh_rate: 70},
                                    VideoMode{colors: 16, screen_width: 640, screen_height: 480, refresh_rate: 60}];

pub struct VDP {
    cursor: Cursor,
    sdl_context: Sdl,
    canvas: Canvas<Window>,
    tx: Sender<u8>,
    rx: Receiver<u8>,
    foreground_color: sdl2::pixels::Color,
    background_color: sdl2::pixels::Color,
    test_color: sdl2::pixels::Color,
    cursor_active: bool,
    cursor_last_change: Instant,
    vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
    last_vsync: Instant,
    current_video_mode: usize,
    text: Vec<Vec<u8>>,
    scale: f32,
}

impl VDP {
    pub fn new(tx: Sender<u8>, rx: Receiver<u8>, vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>) -> Result<VDP, String> {
        let mode =  &VIDEO_MODES[1];
        let scale = 2.0f32;

        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;
    
        let mut window = video_subsystem
            .window("agon-light-emulator", mode.screen_width * scale as u32, mode.screen_height * scale as u32)
            .position_centered()
            .opengl()
            .build()
            .map_err(|e| e.to_string())?;
    
        let mut canvas = window.into_canvas().present_vsync().build().map_err(|e| e.to_string())?;
        canvas.set_scale(scale, scale);
     
        Ok(VDP {
            cursor: Cursor::new(mode.screen_width as i32, mode.screen_height as i32, 8, 8),
            sdl_context: sdl_context,
            canvas: canvas,
            tx: tx,
            rx: rx,
            foreground_color: Color::RGB(255, 255, 255),
            background_color: Color::RGB(0, 0, 0),
            test_color: Color::RGB(255, 0, 0),
            cursor_active: false,
            cursor_last_change: Instant::now(),
            vsync_counter: vsync_counter,
            last_vsync: Instant::now(),
            current_video_mode: 1,
            text: Vec::new(),
            scale: scale,
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.change_mode(1);
        self.bootscreen();
    
        let mut event_pump = self.sdl_context.event_pump()?;
    
        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'running,
                    Event::KeyUp {keycode, keymod, ..}  | Event::KeyDown {keycode, keymod, ..} => {
                        match keycode {
                            Some(keycode) =>
                            {
                                match keycode {
                                    Keycode::LShift | Keycode::RShift | Keycode::LAlt | Keycode::RAlt | Keycode::LCtrl | Keycode::RCtrl | Keycode::CapsLock => (),
                                    Keycode::F1 => self.change_mode(0),
                                    Keycode::F2 => self.change_mode(1),
                                    Keycode::F3 => self.change_mode(2),
                                    Keycode::F4 => self.change_mode(3),
                                    _ => {
                                        let ascii = Self::sdl_keycode_to_mos_keycode(keycode, keymod);
                                        let up = matches!(event, Event::KeyUp{..});
                                        println!("Pressed key:{} with mod:{} ascii:{} up:{}", keycode, keymod, ascii, up);
                                        self.send_key(ascii, up);
                                    }
                                }
                            },
                            None => println!("Invalid key pressed."),
                        }
                    },
                    _ => (),
                }
            }
    
            
            self.do_comms();
            self.blink_cusor();
            // a fake vsync every 16ms
            if self.last_vsync.elapsed().as_millis() > (1000 / VIDEO_MODES[self.current_video_mode].refresh_rate as u128) {
                self.vsync_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                self.last_vsync = Instant::now();
                self.canvas.present();
                self.canvas.set_draw_color(self.background_color);
                self.canvas.clear();
                self.render_text();
            }
        }

        Ok(())
    }

    fn change_mode(&mut self, mode: usize) {
        let video_mode =  &VIDEO_MODES[mode];
        self.cursor.screen_height = video_mode.screen_height as i32;
        self.cursor.screen_width = video_mode.screen_width as i32;
        self.canvas.window_mut().set_size(video_mode.screen_width * self.scale as u32, video_mode.screen_height * self.scale as u32);
        self.current_video_mode = mode;
        self.text.resize((video_mode.screen_width / self.cursor.font_width as u32) as usize, vec![0; (video_mode.screen_height / self.cursor.font_height as u32) as usize]);
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
    
    fn render_text(&mut self)
    {
        for (y, row) in self.text.iter().enumerate() {
            for (x, ascii)  in row.iter().enumerate() {
                if *ascii == 0x0 {
                    continue;
                }
                //println!("Render {:#02X?}", ascii);
                let start = (8 * *ascii as u32) as usize;
                let end = start+8 as usize;
                let mut points = Self::get_points_from_font(FONT_BYTES[start..end].to_vec());
                self.canvas.set_draw_color(self.foreground_color);
                let video_mode = &VIDEO_MODES[self.current_video_mode];
                for point in points.iter_mut() {
                    point.x += x as i32 * self.cursor.font_width;
                    point.y += y as i32 * self.cursor.font_height;
                }
                self.canvas.draw_points(&points[..]);
            }
        }
    }

    fn set_text(&mut self, ascii: u8) {
        if ascii >= 32
        {
            let shifted_ascii = ascii - 32;
            let x: usize = (self.cursor.position_x / self.cursor.font_width) as usize;
            let y: usize = (self.cursor.position_y / self.cursor.font_height) as usize;
            let stride = VIDEO_MODES[self.current_video_mode].screen_width / self.cursor.font_width as u32;
            self.text[y][x] = shifted_ascii;
        }
    }

    pub fn bootscreen(&mut self) {
        let boot_message = "Agon Quark VDP Version 1.03";
        for byte in boot_message.as_bytes() {
            self.set_text(*byte);
            self.cursor.right();
        }
        self.cursor.down();
        self.cursor.home();
    }

    pub fn blink_cusor(&mut self) {
        if self.cursor_last_change.elapsed().as_millis() < 500 {
            return;
        }
        if self.cursor_active {
            self.canvas.set_draw_color(self.foreground_color);
        } else {
            self.canvas.set_draw_color(self.background_color);
        }
        self.cursor_active = !self.cursor_active;
        self.canvas.fill_rect(Rect::new(self.cursor.position_x as i32, self.cursor.position_y as i32, 8, 8));
        self.cursor_last_change = Instant::now();
    }


    pub fn backspace(&mut self) {
        self.cursor.left();
        self.set_text(b' ');
    }

    
    pub fn cls(&mut self) {
        self.canvas.set_draw_color(self.background_color);
        self.canvas.clear();
        self.cursor.position_x = 0;
        self.cursor.position_y = 0;
        for row in &mut self.text {
            for char in row {
                *char = 0;
            }
        }
    }

    pub fn send_key(&self, keycode: u8, up: bool){
        let mut keyboard_packet: Vec<u8> = vec![keycode, 0, 0, up as u8];
		self.send_packet(0x1, keyboard_packet.len() as u8, &mut keyboard_packet);
    }

    fn send_cursor_position(&self) {
        let mut cursor_position_packet: Vec<u8> = vec![(self.cursor.position_x / self.cursor.font_width) as u8,
        (self.cursor.position_y / self.cursor.font_height) as u8];
        self.send_packet(0x02, cursor_position_packet.len() as u8, &mut cursor_position_packet);	
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
    

    pub fn do_comms(&mut self) {
        match self.rx.try_recv() {
            Ok(n) => {
                match n {
                    n if n >= 0x20 && n != 0x7F => {
                        println!("Received character: {}", n as char);
                        self.set_text(n);
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
                                    0x82 => {
                                        println!("Send Cursor Position");
                                        self.send_cursor_position();
                                    },
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
                                            VIDEO_MODES[self.current_video_mode].colors
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
    }
}
