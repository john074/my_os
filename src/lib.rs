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
mod framebuffer;
mod mouse;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate alloc;

#[unsafe(no_mangle)]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
	//vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Green);
	interrupts::init();
	cpu::enable_nxe_bit();
	cpu::enable_write_protect_bit();
	memory::init(multiboot_information_address);
	let mut framebuffer = unsafe { framebuffer::init(multiboot_information_address) };
	unsafe { framebuffer::FRAME_BUFFER_PTR = &mut framebuffer as *mut framebuffer::Framebuffer; }
	let mut mouse = mouse::init_mouse();
	unsafe { mouse::MOUSE_PTR = &mut mouse as *mut mouse::Mouse; }
	let executor = Box::new(multitasking::Executor::new());
	let ata = fat32::AtaDevice::new();
	let mut boxed_ata = Box::new(ata);
	let mut fs = fat32::mount_fat32(boxed_ata).unwrap();
	//vga_buffer::clear_screen();
	unsafe {
	    multitasking::EXECUTOR_PTR = Box::into_raw(executor);
	    fat32::FS_PTR = &mut fs as *mut fat32::FAT32Volume;
		(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(framebuffer::gui_loop(&mut *framebuffer::FRAME_BUFFER_PTR)));
	    //(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(keyboard::print_keypresses()));
	    //(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(start_shell()));
	    (*multitasking::EXECUTOR_PTR).run();
	}
}

async fn start_shell() {
	let fs = unsafe { &mut *fat32::FS_PTR };
	let data = fs.read_file("/SOMNIA").unwrap();	
	fat32::load_elf_and_jump(&data);
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

