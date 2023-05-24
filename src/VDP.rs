use core::panic;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
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

static COLOUR_LOOKUP: [sdl2::pixels::Color; 64] = [
	Color::RGB(0x00, 0x00, 0x00), Color::RGB(0x00, 0x00, 0x55), Color::RGB(0x00, 0x00, 0xAA), Color::RGB(0x00, 0x00, 0xFF),
	Color::RGB(0x00, 0x55, 0x00), Color::RGB(0x00, 0x55, 0x55), Color::RGB(0x00, 0x55, 0xAA), Color::RGB(0x00, 0x55, 0xFF),
	Color::RGB(0x00, 0xAA, 0x00), Color::RGB(0x00, 0xAA, 0x55), Color::RGB(0x00, 0xAA, 0xAA), Color::RGB(0x00, 0xAA, 0xFF),
	Color::RGB(0x00, 0xFF, 0x00), Color::RGB(0x00, 0xFF, 0x55), Color::RGB(0x00, 0xFF, 0xAA), Color::RGB(0x00, 0xFF, 0xFF),
	Color::RGB(0x55, 0x00, 0x00), Color::RGB(0x55, 0x00, 0x55), Color::RGB(0x55, 0x00, 0xAA), Color::RGB(0x55, 0x00, 0xFF),
	Color::RGB(0x55, 0x55, 0x00), Color::RGB(0x55, 0x55, 0x55), Color::RGB(0x55, 0x55, 0xAA), Color::RGB(0x55, 0x55, 0xFF),
	Color::RGB(0x55, 0xAA, 0x00), Color::RGB(0x55, 0xAA, 0x55), Color::RGB(0x55, 0xAA, 0xAA), Color::RGB(0x55, 0xAA, 0xFF),
	Color::RGB(0x55, 0xFF, 0x00), Color::RGB(0x55, 0xFF, 0x55), Color::RGB(0x55, 0xFF, 0xAA), Color::RGB(0x55, 0xFF, 0xFF),
	Color::RGB(0xAA, 0x00, 0x00), Color::RGB(0xAA, 0x00, 0x55), Color::RGB(0xAA, 0x00, 0xAA), Color::RGB(0xAA, 0x00, 0xFF),
	Color::RGB(0xAA, 0x55, 0x00), Color::RGB(0xAA, 0x55, 0x55), Color::RGB(0xAA, 0x55, 0xAA), Color::RGB(0xAA, 0x55, 0xFF),
	Color::RGB(0xAA, 0xAA, 0x00), Color::RGB(0xAA, 0xAA, 0x55), Color::RGB(0xAA, 0xAA, 0xAA), Color::RGB(0xAA, 0xAA, 0xFF),
	Color::RGB(0xAA, 0xFF, 0x00), Color::RGB(0xAA, 0xFF, 0x55), Color::RGB(0xAA, 0xFF, 0xAA), Color::RGB(0xAA, 0xFF, 0xFF),
	Color::RGB(0xFF, 0x00, 0x00), Color::RGB(0xFF, 0x00, 0x55), Color::RGB(0xFF, 0x00, 0xAA), Color::RGB(0xFF, 0x00, 0xFF),
	Color::RGB(0xFF, 0x55, 0x00), Color::RGB(0xFF, 0x55, 0x55), Color::RGB(0xFF, 0x55, 0xAA), Color::RGB(0xFF, 0x55, 0xFF),
	Color::RGB(0xFF, 0xAA, 0x00), Color::RGB(0xFF, 0xAA, 0x55), Color::RGB(0xFF, 0xAA, 0xAA), Color::RGB(0xFF, 0xAA, 0xFF),
	Color::RGB(0xFF, 0xFF, 0x00), Color::RGB(0xFF, 0xFF, 0x55), Color::RGB(0xFF, 0xFF, 0xAA), Color::RGB(0xFF, 0xFF, 0xFF),
];

struct VideoMode{
    colors: u8,
    screen_width: u32,
    screen_height: u32,
    refresh_rate: u8,
    palette: &'static[&'static Color],
}

static PALETTE_2: [&'static Color; 2] = [&COLOUR_LOOKUP[0x00], &COLOUR_LOOKUP[0x3F]];
static PALETTE_16: [&'static Color; 16] = [&COLOUR_LOOKUP[0x00], &COLOUR_LOOKUP[0x20], &COLOUR_LOOKUP[0x08], &COLOUR_LOOKUP[0x28], &COLOUR_LOOKUP[0x02], &COLOUR_LOOKUP[0x22], &COLOUR_LOOKUP[0x0A], &COLOUR_LOOKUP[0x2A], &COLOUR_LOOKUP[0x15], &COLOUR_LOOKUP[0x30], &COLOUR_LOOKUP[0x0C], &COLOUR_LOOKUP[0x3C], &COLOUR_LOOKUP[0x03], &COLOUR_LOOKUP[0x33], &COLOUR_LOOKUP[0x0F], &COLOUR_LOOKUP[0x3F]];
static PALETTE_64: [&'static Color; 64] = [&COLOUR_LOOKUP[0x00], &COLOUR_LOOKUP[0x20], &COLOUR_LOOKUP[0x08], &COLOUR_LOOKUP[0x28], &COLOUR_LOOKUP[0x02], &COLOUR_LOOKUP[0x22], &COLOUR_LOOKUP[0x0A], &COLOUR_LOOKUP[0x2A], &COLOUR_LOOKUP[0x15], &COLOUR_LOOKUP[0x30], &COLOUR_LOOKUP[0x0C], &COLOUR_LOOKUP[0x3C], &COLOUR_LOOKUP[0x03], &COLOUR_LOOKUP[0x33], &COLOUR_LOOKUP[0x0F], &COLOUR_LOOKUP[0x3F], &COLOUR_LOOKUP[0x01], &COLOUR_LOOKUP[0x04], &COLOUR_LOOKUP[0x05], &COLOUR_LOOKUP[0x06], &COLOUR_LOOKUP[0x07], &COLOUR_LOOKUP[0x09], &COLOUR_LOOKUP[0x0B], &COLOUR_LOOKUP[0x0D], &COLOUR_LOOKUP[0x0E], &COLOUR_LOOKUP[0x10], &COLOUR_LOOKUP[0x11], &COLOUR_LOOKUP[0x12], &COLOUR_LOOKUP[0x13], &COLOUR_LOOKUP[0x14], &COLOUR_LOOKUP[0x16], &COLOUR_LOOKUP[0x17], &COLOUR_LOOKUP[0x18], &COLOUR_LOOKUP[0x19], &COLOUR_LOOKUP[0x1A], &COLOUR_LOOKUP[0x1B], &COLOUR_LOOKUP[0x1C], &COLOUR_LOOKUP[0x1D], &COLOUR_LOOKUP[0x1E], &COLOUR_LOOKUP[0x1F], &COLOUR_LOOKUP[0x21], &COLOUR_LOOKUP[0x23], &COLOUR_LOOKUP[0x24], &COLOUR_LOOKUP[0x25], &COLOUR_LOOKUP[0x26], &COLOUR_LOOKUP[0x27], &COLOUR_LOOKUP[0x29], &COLOUR_LOOKUP[0x2B], &COLOUR_LOOKUP[0x2C], &COLOUR_LOOKUP[0x2D], &COLOUR_LOOKUP[0x2E], &COLOUR_LOOKUP[0x2F], &COLOUR_LOOKUP[0x31], &COLOUR_LOOKUP[0x32], &COLOUR_LOOKUP[0x34], &COLOUR_LOOKUP[0x35], &COLOUR_LOOKUP[0x36], &COLOUR_LOOKUP[0x37], &COLOUR_LOOKUP[0x38], &COLOUR_LOOKUP[0x39], &COLOUR_LOOKUP[0x3A], &COLOUR_LOOKUP[0x3B], &COLOUR_LOOKUP[0x3D], &COLOUR_LOOKUP[0x3E]];

static VIDEO_MODES: [VideoMode; 4] = [VideoMode{colors: 2, screen_width: 1024, screen_height: 768, refresh_rate: 60, palette: &PALETTE_2},
                                    VideoMode{colors: 16, screen_width: 512, screen_height: 384, refresh_rate: 60, palette: &PALETTE_16},
                                    VideoMode{colors: 64, screen_width: 320, screen_height: 240, refresh_rate: 75, palette: &PALETTE_64},
                                    VideoMode{colors: 16, screen_width: 640, screen_height: 480, refresh_rate: 60, palette: &PALETTE_16}];

pub struct VDP<'a> {
    cursor: Cursor,
    canvas: Canvas<Window>,
    texture: Texture<'a>,
    texture_creator: &'a TextureCreator<WindowContext>,
    tx: Sender<u8>,
    rx: Receiver<u8>,
    foreground_color: sdl2::pixels::Color,
    background_color: sdl2::pixels::Color,
    graph_color: sdl2::pixels::Color,
    cursor_active: bool,
    cursor_last_change: Instant,
    vsync_counter: std::sync::Arc<std::sync::atomic::AtomicU32>,
    last_vsync: Instant,
    current_video_mode: &'static VideoMode,
    logical_coords: bool,
    p1: Point,
    p2: Point,
    p3: Point,
    graph_origin: Point,
    FONT_DATA: Vec<u8>
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
            graph_color: Color::RGB(255, 255, 255),
            cursor_active: false,
            cursor_last_change: Instant::now(),
            vsync_counter: vsync_counter,
            last_vsync: Instant::now(),
            current_video_mode: mode,
            FONT_DATA: FONT_BYTES.to_vec(),
            logical_coords: true,
            p1: Point::new(0,0),
            p2: Point::new(0,0),
            p3: Point::new(0,0),
            graph_origin: Point::new(0,0),
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
        self.p1.x = 0;
        self.p1.y = 0;
        self.p2.x = 0;
        self.p2.y = 0;
        self.p3.x = 0;
        self.p3.y = 0;
        self.graph_origin.x = 0;
        self.graph_origin.y = 0;
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
            let mut points = Self::get_points_from_font(self.FONT_DATA[start..end].to_vec());
            
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
    
    pub fn clg(&mut self) {
        self.canvas.with_texture_canvas(&mut self.texture, |texture_canvas| {
            texture_canvas.set_draw_color(self.background_color);
            texture_canvas.clear();
        });
    }

    pub fn color(&mut self, c: u8) {
        
    }

    pub fn gcolor(&mut self, m: u8, c: u8) {
        
    }    

    pub fn scale(&self, p: Point) -> Point {
        if self.logical_coords
        {
            Point::new(p.x*self.cursor.screen_width/1280, p.y*self.cursor.screen_height/1024)
        }
        else
        {
            p
        }
    }

    pub fn translate(&self, p: Point) -> Point {
        if self.logical_coords
        {
            Point::new(p.x+self.graph_origin.x, self.cursor.screen_height - p.y - self.graph_origin.y)
        }
        else
        {
            Point::new(p.x+self.graph_origin.x, p.y+self.graph_origin.y)
        }
    }

    // Return the x coorinates for each y coordinate of the line from point
    // top to the point bot. top.x and bot.x are included unless we have a
    // horizontal line, in which case we have only bot.x
    pub fn line_xcoords(top : Point, bot : Point) -> Vec<i32> {
        let mut xc = Vec::<i32>::new();
        let dy = (bot.y - top.y).abs();
        let dx = (top.x - bot.x).abs();
        if dy == 0 {
            xc.push(bot.x)
        } else {
            let mut y = top.y;
            if (dx > dy) {
                let mut t = -dx/2;
                let mut y = top.y;
                // 'horizontal line', iterate over x.
                xc.push(top.x);
                if top.x < bot.x {
                    for x in top.x..=bot.x {
                        t = t+dy;
                        if (t>0) {
                            t=t-dx;
                            if (x!=top.x && x!=bot.x && y!=bot.y && y!=top.y) { 
                                xc.push(x);
                            }
                            y=y+1;
                        }
                    }
                } else {
                    for x in (bot.x..=top.x).rev() {
                        t = t+dy;
                        if (t>0) {
                            t=t-dx;
                            if (x!=top.x && x!=bot.x && y!=bot.y && y!=top.y) { 
                                xc.push(x);
                            }
                            y=y+1;
                        }
                    }
                }
                xc.push(bot.x);
            } else {
                // 'vertical line', iterate over y, assume top.y < bot.y
                let mut t = -dy/2;
                let mut x = top.x;
                for y in top.y..=bot.y {
                    xc.push(x);
                    t += dx;
                    if t>0 {
                        if top.x > bot.x {
                            x-=1;
                        } else {
                            x+=1;
                        }
                        t=t-dy;
                    }
                }
            }
        }
        println!("returned list size = {} dy={}",xc.len(),bot.y-top.y);
        if (xc.len() as i32) < bot.y-top.y+1 {
            xc.push(bot.x);
            println!("unexpected coordinate list too short adding one");
        } else if (xc.len() as i32) > bot.y-top.y+1 {
            println!("unexpected coordinate list too long");
        }
        xc
    }
    
    pub fn plot(&mut self, mode: u8, x: i16, y: i16) {
        self.p3 = self.p2;
        self.p2 = self.p1;
        self.p1 = self.translate(self.scale(Point::new(x as i32,y as i32)));
        self.canvas.with_texture_canvas(&mut self.texture, |texture_canvas| {
            texture_canvas.set_draw_color(self.graph_color);
            match mode {
                4 => {println!("MOVETO");},
                5 => {
                    println!("LINETO");
                    texture_canvas.draw_line(self.p1,self.p2);
                },
                64..=71 => {
                    println!("PLOTDOT");
                    texture_canvas.draw_point(self.p1);
                },
                80..=87 => {
                    println!("TRIANGLE");
                    // Todo should be a filled triangle. just outline is shown.
                    //texture_canvas.draw_line(self.p1,self.p2);
                    //texture_canvas.draw_line(self.p2,self.p3);
                    //texture_canvas.draw_line(self.p3,self.p1);
                    let mut ptop : Point = self.p1;
                    let mut pmid : Point = self.p2;
                    let mut pbot : Point = self.p3;
                    // Order the points from top to bottom.
                    if (ptop.y > pmid.y)
                    {
                        (ptop,pmid) = (pmid,ptop);
                    }
                    if (ptop.y > pbot.y)
                    {
                        (ptop,pbot) = (pbot,ptop);
                    }
                    if (pmid.y > pbot.y)
                    {
                        (pmid,pbot) = (pbot,pmid);
                    }
                    println!("Points are {},{}  {},{} {},{}",ptop.x,ptop.y,pmid.x,pmid.y,pbot.x,pbot.y);
                    let xv1 = Self::line_xcoords(ptop, pbot);
                    let mut xv2 = Self::line_xcoords(ptop, pmid);
                    xv2.append((&mut Self::line_xcoords(pmid,pbot)[1..].to_vec()));
                    let mut y = ptop.y;
                    for (i,x1) in xv1.iter().enumerate() {
                        let x2 = xv2[i];
                        texture_canvas.draw_line(Point::new(*x1,y),Point::new(x2,y));
                        y += 1;
                    }
                },
                144..=151 => {
                    let mut r: f32 = 0.0;
                    if (mode < 148) {
                        r = ((self.p1.x * self.p1.x + self.p1.y * self.p1.y) as f32).sqrt();
                    } else {
                        let rx = self.p1.x - self.p2.x;
                        let ry = self.p1.y - self.p2.y;
                        r = ((rx*rx + ry*ry) as f32).sqrt();
                    }
                    println!("Circle at {},{} radius {}",self.p2.x, self.p2.y,r);
                    let pstart = Point::new(self.p2.x + (r as i32), self.p2.y);
                    let mut pold = pstart;
                    let mut pnew = pold;
                    // suboptimal implementaion of circle.
                    for i in 1..32 {
                        let angle = (i as f32) * 6.28318531 / 32.0;
                        pold = pnew;
                        pnew = Point::new(self.p2.x + ((r*angle.cos()) as i32),
                                          self.p2.y + ((r*angle.sin()) as i32));
                        texture_canvas.draw_line(pold,pnew);                    
                    }
                    texture_canvas.draw_line(pnew,pstart);
                },
                _ => {println!("Unsupported plot mode");}
            }
        });        
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

    fn read_byte(&mut self) -> u8 {
        self.rx.recv().unwrap()
    }

    fn try_read_byte(&mut self) -> Result<u8, TryRecvError> {
        self.rx.try_recv()
    }

    fn read_word(&mut self) -> i16 {
        i16::from_le_bytes([self.rx.recv().unwrap(), self.rx.recv().unwrap()])
    } 

    fn do_comms(&mut self) {
        match self.try_read_byte() {
            Ok(n) => {
                match n {
                    n if n >= 0x20 && n != 0x7F => {
                        println!("Received character: {}", n as char);
                        self.render_char(n);
                        self.cursor.right();
                        self.check_scrolling_needed();
                    },
                    0x08 => {println!("Cursor left."); self.cursor.left();},
                    0x09 => {println!("Cursor right."); self.cursor.right();},
                    0x0A => {
                        println!("Cursor down.");
                        self.cursor.down();
                        self.check_scrolling_needed();
                    },
                    0x0B => {println!("Cursor up."); self.cursor.up();},
                    0x0C => {
                        println!("CLS.");
                        self.cls();
                    },
                    0x0D => {println!("Cursor home."); self.cursor.home();},
                    0x0E => {println!("PageMode ON?");},
                    0x0F => {println!("PageMode OFF?");},
                    0x10 => {
                        println!("CLG");
                        self.clg();
                    },
                    0x11 => {
                        let c = self.read_byte();
                        self.color(c);
                        println!("COLOUR {}",c);
                        if c < 128 {
                            self.foreground_color = *self.current_video_mode.palette[c as usize % self.current_video_mode.palette.len()];
                        } else {
                            self.background_color = *self.current_video_mode.palette[c as usize % self.current_video_mode.palette.len()];
                        }
                    },
                    0x12 => {
                        let m = self.read_byte();
                        let c = self.read_byte();
                        self.gcolor(m,c);
                        println!("GCOL {},{}",m,c);
                        self.graph_color = *self.current_video_mode.palette[c as usize % self.current_video_mode.palette.len()];
                    },
                    0x13 => {
                        let l = self.read_byte();
                        let p = self.read_byte();
                        let r = self.read_byte();
                        let g = self.read_byte();
                        let b = self.read_byte();
                        println!("Define Logical Colour?: l:{} p:{} r:{} g:{} b:{}", l, p, r, g, b);
                    },
                    0x16 => {
                        println!("MODE.");
                        let mode = self.read_byte();
                        self.change_mode(mode.into());
                        self.send_mode_information();
                    },
                    0x17 => {
                        println!("VDU23.");
                        match self.read_byte() {
                            0x00 => {
                                println!("Video System Control.");
                                match self.read_byte() {
                                    0x80 => {
                                        println!("VDP_GP.");
                                        let mut packet = Vec::new();
                                        packet.push(self.read_byte());
                                        self.send_packet(0x00, packet.len() as u8, &mut packet);
                                    },
                                    0x81 => println!("VDP_KEYCODE"),
                                    0x82 => {
                                        println!("Send Cursor Position");
                                        self.send_cursor_position();
                                    },
                                    0x85 => {
                                        let channel = self.read_byte();
                                        let waveform = self.read_byte();
                                        let volume = self.read_byte();
                                        let frequency = self.read_word();
                                        let duration = self.read_word();
                                        println!("VDP_AUDIO?: channel:{} waveform:{} volume:{} frequency:{} duration:{}", channel, waveform, volume, frequency, duration);
                                    }
                                    0x86 => {
                                        println!("Mode Information");
                                        self.send_mode_information();
                                    },
                                    0xC0 => {
                                        let b = self.read_byte();
                                        self.logical_coords = (b!=0);
                                        println!("Set logical coords {}\n",self.logical_coords);
                                    }
                                    n => println!("Unknown VSC command: {:#02X?}.", n),
                                }
                            },
                            0x01 => println!("Cursor Control?"),
                            0x07 => println!("Scroll?"),
                            0x1B => println!("Sprite Control?"),
                            n if n>=32 => {
                                    for i in 0..8 {
                                        let b =  self.read_byte();
                                        self.FONT_DATA[((n-32)as u32*8+i) as usize] = b;
                                    }
                                    println!("Redefine char bitmap: {}.", n);
                                },
                            n => { println!("Unknown VDU command: {:#02X?}.", n);}
                        }
                    },
                    0x19 => {
                        let mode = self.read_byte();
                        let x = self.read_word();
                        let y = self.read_word();
                        println!("PLOT {},{},{}",mode,x,y);
                        self.plot(mode,x,y);
                    },
                    0x1D => {
                        let x = self.read_word() as i32;
                        let y = self.read_word() as i32;
                        if x>= 0 && y>= 0 {
                            self.graph_origin=self.scale(Point::new(x,y));
                        }
                        println!("Graph origin {},{}",x,y);
                    },
                    0x1E => {println!("Home."); self.cursor.home();},
                    0x1F => {
                        let x = self.read_byte() as i32 * self.cursor.font_width;
                        let y = self.read_byte() as i32 * self.cursor.font_height;
                        println!("TAB({},{})",x,y);
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

    fn check_scrolling_needed(&mut self) {
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
