use lazy_static::lazy_static;
use volatile::Volatile;
use spin::Mutex;
use core::fmt;

use crate::carriage;

lazy_static! {
	pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
		column_position: 0,
		color_code: ColorCode::new(Color::Yellow, Color::Black),
		buffer: unsafe { &mut *(0xb8000 as *mut Buffer)},
	});
			
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
	Black = 0,
	Blue = 1,
	Green = 2,
	Cyan = 3,
	Red = 4,
	Magneta = 5,
	Brown = 6,
	LightGray = 7,
	DarkGray = 8,
	LightBlue = 9,
	LightGreen = 10,
	LightCyan = 11,
	LightRed = 12,
	Pink = 13,
	Yellow = 14,
	White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
	fn new(foreground:Color, background:Color) -> ColorCode {
		ColorCode((background as u8) << 4 | (foreground as u8))
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
	ascii_char: u8,
	color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer{
	chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
	column_position: usize,
	color_code: ColorCode,
	buffer: &'static mut Buffer,
}

impl Writer {
	pub fn write_string(&mut self, s: &str)
	{
		for byte in s.bytes() {
			match byte {
				0x20..=0x7e | b'\n' => self.write_byte(byte),
				_ => self.write_byte(0xfe),
			}
		}
	}

	pub fn write_byte(&mut self, byte: u8){
		match byte {
			b'\n' => self.new_line(),
			byte => {
				if self.column_position >= BUFFER_WIDTH {
					self.new_line();
				}

				let row = BUFFER_HEIGHT - 1;
				let col = self.column_position;

				let color_code = self.color_code;
				self.buffer.chars[row][col].write(ScreenChar {
					ascii_char: byte,
					color_code,
				});
				self.column_position += 1;
				self.update_cursor();
			}
		}
	}

	pub fn rm_char(&mut self) {
	    if self.column_position == 1 && self.buffer.chars[BUFFER_HEIGHT - 1][0].read().ascii_char == b'>' {
	        return;
	    }
	   	if self.column_position > 0 {
	        self.column_position -= 1;
	        let row = BUFFER_HEIGHT - 1;
	        let col = self.column_position;
	        self.buffer.chars[row][col].write(ScreenChar {
	            ascii_char: b' ',
	            color_code: self.color_code,
	        });
	        self.update_cursor();
	    } else {
	        for row in (1..BUFFER_HEIGHT).rev() {
	            for col in 0..BUFFER_WIDTH {
	                let character = self.buffer.chars[row - 1][col].read();
	                self.buffer.chars[row][col].write(character);
	            }
	        }
	        self.clear_row(0);
	    
	        self.column_position = BUFFER_WIDTH;
	        self.update_cursor();
	    }
	}

	fn update_cursor(&self) {
		let position = (BUFFER_HEIGHT - 1) * BUFFER_WIDTH + self.column_position;
	    carriage::set_cursor_position(position as u16);
	}

	pub fn set_color(&mut self, foreground: Color, background: Color) {
		self.color_code = ColorCode::new(foreground, background);
	}
	
	pub fn set_foreground_color(&mut self, foreground: Color) {
		self.color_code = ColorCode::new(foreground, Color::Black);
	}

	fn new_line(&mut self) {
		for row in 1..BUFFER_HEIGHT {
			for col in 0..BUFFER_WIDTH {
				let character = self.buffer.chars[row][col].read();
				self.buffer.chars[row - 1][col].write(character);
			}
		}
		self.clear_row(BUFFER_HEIGHT - 1);
		self.column_position = 0;
	}

	fn clear_row(&mut self, row: usize){
		let blank = ScreenChar {
			ascii_char: b' ',
			color_code: self.color_code,
		};
		for col in 0..BUFFER_WIDTH {
			self.buffer.chars[row][col].write(blank);
		}
	}
}

impl fmt::Write for Writer {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.write_string(s);
		Ok(())
	}
}

pub fn rm_char() {
	WRITER.lock().rm_char();
}

#[macro_export]
macro_rules! print {
	($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
	() => ($crate::print!("\n"));
	($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debug {
	($($arg:tt)*) => {{
		use core::fmt::Write;
		use x86_64::instructions::interrupts;

		interrupts::without_interrupts(|| {
			let mut writer = $crate::vga_buffer::WRITER.lock();
			writer.set_foreground_color($crate::vga_buffer::Color::Yellow);
			writeln!(writer, "{}", format_args!($($arg)*)).ok();
			writer.set_foreground_color($crate::vga_buffer::Color::Green);
		});
	}};
}


#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
	use core::fmt::Write;
	use x86_64::instructions::interrupts;

	interrupts::without_interrupts(|| {
		WRITER.lock().write_fmt(args).unwrap();		
	});
}

pub fn clear_screen() {
	for _ in 0..BUFFER_HEIGHT {
		println!("");
	}
}
