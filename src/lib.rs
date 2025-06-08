#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]

use core::panic::PanicInfo;

mod vga_buffer;
mod interrupts;
mod gdt;
mod carriage;
mod memory;
mod cpu;
mod time;
mod std;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate alloc;


#[unsafe(no_mangle)]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Green);

	interrupts::init();
	time::sleep(1000);
	cpu::enable_nxe_bit();
	cpu::enable_write_protect_bit();
	memory::init(multiboot_information_address);
	time::sleep(2000);
	vga_buffer::clear_screen();

	println!("*");

	let mut heap_t = alloc::boxed::Box::new(42);
	let heap_test2 = alloc::boxed::Box::new("Hellow");
	let mut vec_test = vec![1, 2, 3, 4, 5];
	*heap_t -= 15;
	vec_test[3] = 42;

	for i in 0..100 {
		alloc::boxed::Box::new(42);
		alloc::boxed::Box::new(42000);
	}

	for i in 0..10000 {
		format!("Some string!");
	}

	println!("{:?}, {:?}", heap_t, heap_test2);
	
	print!("Hello World!\n>");
	
	hlt_loop();
}

pub fn hlt_loop() -> ! {
	loop {
		cpu::hlt();
	}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Red);
	println!("{}", _info);
	hlt_loop();
}

