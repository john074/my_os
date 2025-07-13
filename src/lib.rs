#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]
#![feature(naked_functions)]

#![feature(ptr_metadata)]

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
	let executor_ptr = &mut executor as *mut multitasking::Executor;
	executor.spawn(multitasking::Task::new(cfs(executor_ptr)));

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


async fn cfs(executor: *mut multitasking::Executor) {
	let mut ata = fs::AtaDevice::new();
	let mut fs = fs::Fat12Fs::new(ata);

	let mut program_buf = [0u8; 65536];
	unsafe{
		let size = fs.read_file(b"APP        ", &mut program_buf).unwrap();	
		fs::load_elf_and_jump(&program_buf[..size], executor);
	}
	
	//tfs(fs).await;
}

async fn tfs<D: fs::BlockDevice>(mut fs: fs::Fat12Fs<D>) {
	let filename = *b"TEST2   TXT";
	let contents = b"Hello from Rust!\n";
	fs.write_file(&filename, contents);

	println!("file 'TEST2.TXT' created");
	println!("All files:");
	fs.list_files();

	fs.delete_file(b"TEST    TXT");
	println!("file 'TEST.TXT' deleted");
	println!("All files:");
	fs.list_files();
	
	let mut name = *b"HELLO   TXT";
	let mut buffer = [0u8; 512];
	
	if let Some(len) = fs.read_file(&name, &mut buffer) {
	    print!("Read file 'HELLO.TXT': {}\n", core::str::from_utf8(&buffer[..len]).unwrap());
	} else {
	    print!("File not found\n");
	}

	name = *b"TEST    TXT";
	
	if let Some(len) = fs.read_file(&name, &mut buffer) {
		print!("Read file 'TEST.TXT': {}\n", core::str::from_utf8(&buffer[..len]).unwrap());
	} else {
	    print!("File not found\n");
	}

	name = *b"TEST2   TXT";
		
	if let Some(len) = fs.read_file(&name, &mut buffer) {
		print!("Read file 'TEST2.TXT': {}\n", core::str::from_utf8(&buffer[..len]).unwrap());
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

