#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]
#![feature(naked_functions)]

#![feature(ptr_metadata)]

use core::panic::PanicInfo;
use alloc::boxed::Box; 
use alloc::vec::Vec;

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
mod fat32;
mod fs;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate alloc;


#[unsafe(no_mangle)]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Green);
	interrupts::init();
	//time::sleep(1000);
	cpu::enable_nxe_bit();
	cpu::enable_write_protect_bit();
	memory::init(multiboot_information_address);
	//time::sleep(1000);
	let mut executor = multitasking::Executor::new();
	let ata = fat32::AtaDevice::new();
	let boxed_ata = Box::new(ata);
	let mut fs = fat32::mount_fat32(boxed_ata).unwrap();
	let executor_ptr = &mut executor as *mut multitasking::Executor;
	//let fs_ptr = &mut fs as *mut fat32::FAT32Volume;
	unsafe { fat32::FS_PTR = &mut fs as *mut fat32::FAT32Volume; }
	vga_buffer::clear_screen();

	executor.spawn(multitasking::Task::new(keyboard::print_keypresses()));
	executor.spawn(multitasking::Task::new(fat32_test(executor_ptr)));
	executor.spawn(multitasking::Task::new(fat32_test_second_programm(executor_ptr)));
	//print!("Hello World!\n>");
	executor.run();
}

async fn fat32_test(executor: *mut multitasking::Executor) {
	let fs = unsafe { &mut *fat32::FS_PTR };
	fs.create_directory("/docs");
	fs.create_directory("/docs/subdocs");
	//fs.create_file("/docs/readme.txt", 0);
	//println!("{:#?}", fs.list_dir("/"));
	//fs.list_dir("/docs");
	//println!("{:#?}", fs.list_dir("/docs/subdocs"));
	//fs.write_file("/docs/readme.txt", b"hello!!!");
	//fs.read_file("/docs/readme.txt");
	//fs.delete_file("/docs/readme.txt");
	//fs.list_dir("/docs");
	//println!("{:#?}", fs.delete_directory("/docs"));
	//println!("{:#?}", fs.list_dir("/"));
	
	let data = fs.read_file("/SOMNIA").unwrap();	
	fat32::load_elf_and_jump(&data, executor);
}

async fn fat32_test_second_programm(executor: *mut multitasking::Executor) {
	let fs = unsafe { &mut *fat32::FS_PTR };
	let data = fs.read_file("/TEST").unwrap();	
	fat32::load_elf_and_jump(&data, executor);
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

