use multiboot2::{ BootInformation, BootInformationHeader };
use core::fmt::{self, Write};
use spin::Mutex;
use lazy_static::lazy_static;

use crate::mouse;
use crate::multitasking;
use crate::fonts;

lazy_static! {
    pub static ref FB_WRITER: Mutex<FramebufferWriter> = Mutex::new(FramebufferWriter::new());
}

#[allow(static_mut_refs)]
pub static mut FRAMEBUFFER: Option<Framebuffer> = None;

static mut DOUBLE_BUF: [u8; 1024 * 768 * 4] = [0; 1024 * 768 * 4];

pub const BLACK:       u32 = 0xFF000000;
pub const WHITE:       u32 = 0xFFFFFFFF;
pub const RED:         u32 = 0xFFFF0000;
pub const GREEN:       u32 = 0xFF00FF00;  
pub const BLUE:        u32 = 0xFF0000FF;
pub const YELLOW:      u32 = 0xFFFFFF00;
pub const CYAN:        u32 = 0xFF00FFFF;
pub const MAGENTA:     u32 = 0xFFFF00FF;
pub const ORANGE:      u32 = 0xFFFF8000;
pub const PURPLE:      u32 = 0xFF800080;
pub const GRAY:        u32 = 0xFF808080;
pub const LIGHT_GRAY:  u32 = 0xFFD3D3D3;
pub const DARK_GRAY:   u32 = 0xFF404040;
pub const BROWN:       u32 = 0xFF8B4513;
pub const PINK:        u32 = 0xFFFFC0CB;

pub struct Framebuffer {
    buf: *mut u8,
    width: usize,
    height: usize,
    pitch: usize,
    bpp: usize,
    double_buf: &'static mut [u8],
}

impl Framebuffer {
    fn put_pixel(&mut self, x: usize, y: usize, color: u32) {
	    if x >= self.width || y >= self.height { return; }
		let offset = y * self.pitch + x * 4; // 4 байта при 32bpp

		self.double_buf[offset] = (color & 0xFF) as u8;          	 // B
		self.double_buf[offset + 1] = ((color >> 8) & 0xFF) as u8;   // G  
		self.double_buf[offset + 2] = ((color >> 16) & 0xFF) as u8;  // R
		self.double_buf[offset + 3] = ((color >> 24) & 0xFF) as u8;  // A
    }

    pub fn get_pixel(&self, x: isize, y: isize) -> u32 {
        if x < 0 || y < 0 || (x as usize) >= self.width || (y as usize) >= self.height {
            return 0;
        }

        let offset = y as usize * self.pitch + (x as usize) * 4;
        let b = self.double_buf[offset] as u32;
        let g = self.double_buf[offset + 1] as u32;
        let r = self.double_buf[offset + 2] as u32;
        let a = self.double_buf[offset + 3] as u32;
        (a << 24) | (r << 16) | (g << 8) | b
    }
    
    pub fn fill_rect(&mut self, x: isize, y: isize, w: isize, h: isize, color: u32) { 
	    let xend = (x + w).min(self.width as isize);
	    let yend = (y + h).min(self.height as isize);
	    for yy in y..yend {
	        for xx in x..xend {
	            self.put_pixel_safe(xx, yy, color);
	        }
	    }
    }

    pub fn draw_line(&mut self, mut x0: isize, mut y0: isize, x1: isize, y1: isize, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.put_pixel_safe(x0, y0, color);
            }
            if x0 == x1 && y0 == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    pub fn put_pixel_safe(&mut self, x: isize, y: isize, color: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height {
            self.put_pixel(x as usize, y as usize, color);
        }
    }

    pub fn draw_circle(&mut self, cx: isize, cy: isize, r: isize, color: u32) {
        let mut x = 0;
        let mut y = r;
        let mut d = 3 - 2 * r;

        while y >= x {
            let points = [
                (cx + x, cy + y),
                (cx - x, cy + y),
                (cx + x, cy - y),
                (cx - x, cy - y),
                (cx + y, cy + x),
                (cx - y, cy + x),
                (cx + y, cy - x),
                (cx - y, cy - x),
            ];

            for &(px, py) in points.iter() {
                self.put_pixel_safe(px, py, color);
            }

            if d < 0 {
                d += 4 * x + 6;
            } else {
                d += 4 * (x - y) + 10;
                y -= 1;
            }
            x += 1;
        }
    }

    pub fn fill_circle(&mut self, cx: isize, cy: isize, r: isize, color: u32) {
        for y in -r..=r {
            for x in -r..=r {
                if x*x + y*y <= r*r {
                    self.put_pixel_safe(cx + x, cy + y, color);
                }
            }
        }
    }

    pub fn draw_triangle(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, x2: isize, y2: isize, color: u32) {
        self.draw_line(x0, y0, x1, y1, color);
        self.draw_line(x1, y1, x2, y2, color);
        self.draw_line(x2, y2, x0, y0, color);
    }

    pub fn fill_triangle(&mut self, mut x0: isize, mut y0: isize, mut x1: isize, mut y1: isize, mut x2: isize, mut y2: isize, color: u32) {
        let min_x = x0.min(x1).min(x2);
        let max_x = x0.max(x1).max(x2);
        let min_y = y0.min(y1).min(y2);
        let max_y = y0.max(y1).max(y2);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let lambda1 = ((y1 - y2)*(x - x2) + (x2 - x1)*(y - y2)) as f32 /
                              ((y1 - y2)*(x0 - x2) + (x2 - x1)*(y0 - y2)) as f32;
                let lambda2 = ((y2 - y0)*(x - x2) + (x0 - x2)*(y - y2)) as f32 /
                              ((y1 - y2)*(x0 - x2) + (x2 - x1)*(y0 - y2)) as f32;
                let lambda3 = 1.0 - lambda1 - lambda2;
                if lambda1 >= 0.0 && lambda2 >= 0.0 && lambda3 >= 0.0 {
                    if x >= 0 && y >= 0 {
                        self.put_pixel_safe(x, y, color);
                    }
                }
            }
        }
    }

    pub fn draw_char(&mut self, x: isize, y: isize, c: char, color: u32) {
        let idx = c as usize;
        if idx >= 128 { return; }
        let bitmap = fonts::FONT8X8_BASIC[idx];
        for (row, bits) in bitmap.iter().enumerate() {
            for col in 0..8 {
                if (bits >> col) & 1 != 0 {
                    self.put_pixel_safe(x + col as isize, y + row as isize, color);
                }
            }
        }
    }

    pub fn draw_string(&mut self, mut x: isize, mut y: isize, text: &str, color: u32) {
        for c in text.chars() {
            match c {
                '\n' => { y += 8; x = 0; },
                _ => {
                    self.draw_char(x, y, c, color);
                    x += 8;
                }
            }
        }
    }

	pub fn draw_frame(&mut self) {
	    unsafe {
	        let src = self.double_buf.as_ptr();
	        let dst = self.buf;
	        core::ptr::copy_nonoverlapping(src, dst, self.pitch * self.height);
	    }
	}

	pub fn fill_screen(&mut self, color: u32) {
		self.fill_rect(0 as isize, 0 as isize, self.width as isize, self.height as isize, color);
	}
}

pub struct FramebufferWriter {
    fb: Option<&'static mut Framebuffer>,
    x: isize,
    y: isize,
    color: u32,
}

impl FramebufferWriter {
    pub const fn new() -> Self {
        Self {
            fb: None,
            x: 0,
            y: 0,
            color: WHITE,
        }
    }

    pub fn set_framebuffer(&mut self, fb: &'static mut Framebuffer) {
        self.fb = Some(fb);
    }

    pub fn set_color(&mut self, color: u32) {
        self.color = color;
    }
}

impl Write for FramebufferWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(fb) = &mut self.fb {
            for c in s.chars() {
                match c {
                    '\n' => {
                        self.y += 8;
                        self.x = 0;
                    }
                    _ => {
                        fb.draw_char(self.x, self.y, c, self.color);
                        self.x += 8;
                        if self.x >= fb.width as isize {
                            self.x = 0;
                            self.y += 8;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

unsafe impl Send for FramebufferWriter {}
unsafe impl Sync for FramebufferWriter {}


#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        use x86_64::instructions::interrupts;

        interrupts::without_interrupts(|| {
            let mut writer = $crate::framebuffer::FB_WRITER.lock();
            writer.write_fmt(format_args!($($arg)*)).unwrap();
        });
    });
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[allow(static_mut_refs)]
pub fn test_colors() {
	let framebuffer = unsafe { FRAMEBUFFER.as_mut().unwrap() };

    framebuffer.fill_rect(0, 0, 100, 100, RED);
    framebuffer.fill_rect(100, 0, 100, 100, GREEN);
    framebuffer.fill_rect(200, 0, 100, 100, BLUE);
    framebuffer.fill_rect(300, 0, 100, 100, WHITE);
    framebuffer.fill_rect(400, 0, 100, 100, YELLOW);
    framebuffer.fill_rect(500, 0, 100, 100, CYAN);
    framebuffer.fill_rect(600, 0, 100, 100, PURPLE);
    framebuffer.fill_rect(0, 100, 100, 100, PINK);
    framebuffer.fill_rect(100, 100, 100, 100, LIGHT_GRAY);
    framebuffer.fill_rect(200, 100, 100, 100, GRAY);
    framebuffer.fill_rect(300, 100, 100, 100, DARK_GRAY);
    framebuffer.fill_rect(400, 100, 100, 100, MAGENTA);
    framebuffer.fill_rect(500, 100, 100, 100, ORANGE);
    framebuffer.fill_rect(600, 100, 100, 100, BROWN);
    framebuffer.draw_string(10, 250, "Test of text. 123456789!,>#", WHITE);
    framebuffer.draw_frame();
}

pub fn draw_house(framebuffer: &mut Framebuffer) {
	framebuffer.fill_screen(CYAN);
	framebuffer.fill_rect(0, (framebuffer.height - 30) as isize, framebuffer.width as isize, 30, GREEN);
	framebuffer.fill_rect(300, (framebuffer.height - 379) as isize, 350, 350, BROWN);
	framebuffer.fill_triangle(280, (framebuffer.height - 379) as isize, 670, (framebuffer.height - 379) as isize, 475, 200, GRAY);
	framebuffer.fill_circle(475, 325, 35, BLACK);
	framebuffer.fill_rect(400, (framebuffer.height - 279) as isize, 150, 150, DARK_GRAY);
	framebuffer.fill_rect(416, (framebuffer.height - 263) as isize, 51, 51, BLUE);
	framebuffer.fill_rect(483, (framebuffer.height - 263) as isize, 51, 51, BLUE);
	framebuffer.fill_rect(416, (framebuffer.height - 196) as isize, 51, 51, BLUE);
	framebuffer.fill_rect(483, (framebuffer.height - 196) as isize, 51, 51, BLUE);
	framebuffer.fill_circle(50, 50, 100, YELLOW);
	framebuffer.draw_line(155, 20, 200, 20, YELLOW);
	framebuffer.draw_line(155, 40, 200, 65, YELLOW);
	framebuffer.draw_line(155, 60, 200, 85, YELLOW);
	framebuffer.draw_line(90, 145, 115, 190, YELLOW);
	framebuffer.draw_line(70, 150, 95, 195, YELLOW);
	framebuffer.draw_line(50, 155, 50, 200, YELLOW);
	framebuffer.draw_frame();
}

#[allow(static_mut_refs)]
pub unsafe fn init(multiboot_information_address: usize) -> &'static mut Framebuffer {
	let boot_info = unsafe{ BootInformation::load(multiboot_information_address as *const BootInformationHeader).unwrap() };
	let fb_tag = boot_info.framebuffer_tag().expect("Framebuffer tag missing").unwrap();
	let buf_size = fb_tag.height() as usize * fb_tag.pitch() as usize;

	let double_buf = unsafe {
	    &mut DOUBLE_BUF[..buf_size.min(DOUBLE_BUF.len())]
	};

	FRAMEBUFFER = Some(Framebuffer {
	    buf: fb_tag.address() as *mut u8,
	    width: fb_tag.width() as usize,
	    height: fb_tag.height() as usize,
	    pitch: fb_tag.pitch() as usize,
	    bpp: fb_tag.bpp() as usize,
	    double_buf,
	});

	FRAMEBUFFER.as_mut().unwrap()
}

#[allow(static_mut_refs)]
pub async fn gui_loop() {
	unsafe {
		let fb = FRAMEBUFFER.as_mut().unwrap();
		let mouse = &mut *mouse::MOUSE_PTR;
		loop {
			if mouse.x != mouse::MOUSE_X || mouse.y != mouse::MOUSE_Y {
				mouse.erase(fb);
				mouse.x = mouse::MOUSE_X;
				mouse.y = mouse::MOUSE_Y;
				mouse.draw(fb);
			}
			fb.draw_frame();
			multitasking::cooperate().await;
		}
	}
}
