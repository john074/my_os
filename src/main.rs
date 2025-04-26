#![no_std]  // no Rust standart lib
#![no_main] // no Rust-level entry points
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

mod vga_buffer;
mod interrupts;
mod gdt;
mod carriage;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Green);
	print!("Hello World!\n>");

	interrupts::init_idt();
	gdt::init();
	unsafe { interrupts::PICS.lock().initialize() };
	x86_64::instructions::interrupts::enable();

	hlt_loop();
}


#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Red);
	println!("{}", _info);
	hlt_loop();
}

pub fn hlt_loop() -> ! {
	loop {
		x86_64::instructions::hlt();
	}
}
