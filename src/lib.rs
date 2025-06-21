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
mod multitasking;
mod keyboard;
mod fs;

#[macro_use]
extern crate bitflags;
extern crate alloc;


#[unsafe(no_mangle)]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Green);
	interrupts::init();
	//time::sleep(1000);
	cpu::enable_nxe_bit();
	cpu::enable_write_protect_bit();
	memory::init(multiboot_information_address);
	//time::sleep(2000);
	vga_buffer::clear_screen();

	let mut executor = multitasking::Executor::new();
	//executor.spawn(multitasking::Task::new(print_a()));
	//executor.spawn(multitasking::Task::new(print_b()));
	executor.spawn(multitasking::Task::new(keyboard::print_keypresses()));
	executor.spawn(multitasking::Task::new(cfs()));
	print!("Hello World!\n>");
	executor.run();
}

async fn print_a() {
	loop {
		print!("a");
		multitasking::cooperate().await;
		time::sleep(1000);
	}
}

async fn print_b() {
	loop {
		print!("b");
		multitasking::cooperate().await;
		time::sleep(1000);
	}
}

async fn cfs() {
	let mut ata = fs::AtaDevice::new();
	let mut fs = fs::Fat12Fs::new(ata);
	
	let name = *b"HELLO   TXT";
	let mut buffer = [0u8; 512];
	
	if let Some(len) = fs.read_file(&name, &mut buffer) {
	    print!("Read file: {}\n", core::str::from_utf8(&buffer[..len]).unwrap());
	} else {
	    print!("File not found\n");
	}
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

