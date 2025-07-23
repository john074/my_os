#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]
#![feature(naked_functions)]

#![feature(ptr_metadata)]

use core::panic::PanicInfo;
use alloc::boxed::Box; 

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
	time::sleep(1000);
	cpu::enable_nxe_bit();
	cpu::enable_write_protect_bit();
	memory::init(multiboot_information_address);
	time::sleep(1000);
	let mut executor = multitasking::Executor::new();
	let ata = fat32::AtaDevice::new();
	let boxed_ata = Box::new(ata);
	let mut fs = fat32::mount_fat32(boxed_ata).unwrap();
	let executor_ptr = &mut executor as *mut multitasking::Executor;
	let fs_ptr = &mut fs as *mut fat32::FAT32Volume;
	vga_buffer::clear_screen();

	executor.spawn(multitasking::Task::new(keyboard::print_keypresses()));
	executor.spawn(multitasking::Task::new(fat32_test(fs_ptr, executor_ptr)));
	executor.spawn(multitasking::Task::new(fat32_test_second_programm(fs_ptr, executor_ptr)));
	print!("Hello World!\n>");
	executor.run();
}

async fn fat32_test(fs_ptr: *mut fat32::FAT32Volume, executor: *mut multitasking::Executor) {
	let fs = unsafe { &mut *fs_ptr };
	fs.create_directory("/docs");
	//fs.create_directory("/docs/subdocs");
	fs.create_file("/docs/readme.txt", 0);
	//println!("{:#?}", fs.list_dir("/"));
	println!("{:#?}", fs.list_dir("/docs"));
	//println!("{:#?}", fs.list_dir("/docs/subdocs"));
	println!("write file: {:#?}", fs.write_file("/docs/readme.txt", b"hello!!!"));
	println!("read file: {:#?}", fs.read_file("/docs/readme.txt"));
	println!("delete file: {:#?}", fs.delete_file("/docs/readme.txt"));
	println!("list /docs: {:#?}", fs.list_dir("/docs"));
	println!("delete /docs: {:#?}", fs.delete_directory("/docs"));
	println!("list /:{:#?}", fs.list_dir("/"));
	
	let data = fs.read_file("/APP").unwrap();	
	fat32::load_elf_and_jump(&data, executor);
}

async fn fat32_test_second_programm(fs_ptr: *mut fat32::FAT32Volume, executor: *mut multitasking::Executor) {
	let fs = unsafe { &mut *fs_ptr };
	let data = fs.read_file("/APP2").unwrap();	
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

