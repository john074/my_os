#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]
#![feature(naked_functions)]
#![feature(ptr_metadata)]

use core::panic::PanicInfo;
use alloc::boxed::Box; 
use alloc::vec::Vec;
use core::sync::atomic::AtomicBool;

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
mod framebuffer;
mod mouse;
mod fonts;
mod gui;
mod tdg;
mod pci;
mod network;

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate alloc;

pub static SYSTEM_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[allow(static_mut_refs)]
#[unsafe(no_mangle)]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
	memory::init(multiboot_information_address);

	let _framebuffer = unsafe { framebuffer::init(multiboot_information_address) };
	framebuffer::FB_WRITER.lock().set_framebuffer(_framebuffer);
	let framebuffer = unsafe { framebuffer::FRAMEBUFFER.as_mut().unwrap() };
	framebuffer::test_colors();

	//time::sleep(3000);
	
	framebuffer.fill_screen(framebuffer::BLACK);
	framebuffer.draw_frame();

	let mut mouse = mouse::init_mouse();
	unsafe { mouse::MOUSE_PTR = &mut mouse as *mut mouse::Mouse; }
	framebuffer.draw_frame();

	unsafe { time::init(); }

	interrupts::init();
	framebuffer.draw_frame();
	
	cpu::enable_nxe_bit();
	framebuffer.draw_frame();
	
	cpu::enable_write_protect_bit();
	framebuffer.draw_frame();
	
	let executor = Box::new(multitasking::Executor::new());
	framebuffer.draw_frame();
	
	let ata = fat32::AtaDevice::new();
	let boxed_ata = Box::new(ata);
	framebuffer.draw_frame();
	let mut fs = fat32::mount_fat32(boxed_ata).unwrap();
	framebuffer.draw_frame();

	framebuffer::draw_background();
	//tdg::mk_bg();
	//tdg::run();

	let mut gui = gui::GuiSystem::new(framebuffer.width as isize, framebuffer.height as isize);
	unsafe { gui::GUI_PTR = &mut gui as *mut gui::GuiSystem }

	let ip_bytes = fs.read_file("ip.txt").unwrap();
	let ip_str = core::str::from_utf8(&ip_bytes[..ip_bytes.len() as usize]).unwrap_or("[invalid utf8]");
	let net_driver;
	if let Some(ip) = network::parse_ip(ip_str) {
		net_driver = network::E1000::init_from_pci(ip);
	} else {
		net_driver = network::E1000::init_from_pci([10,0,0,1]);
	}
	
	unsafe {
	    multitasking::EXECUTOR_PTR = Box::into_raw(executor);
	    fat32::FS_PTR = &mut fs as *mut fat32::FAT32Volume;
		(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(framebuffer::gui_loop(), None));

		let gui = &mut *gui::GUI_PTR;
		let win = gui.create_window("Terminal", 150, 150, 400, 400);

		let term = gui.add_node(
		    win,
		    gui::GuiElement::Terminal(gui::TerminalData {
		        buffer: Vec::new(),
		        cursor_x: 0,
		        cursor_y: 0,
		        text_color: 0xFFFFFF,
		    }),
		    2, 24, 380, 360
		);

		gui.create_taskbar();

		//(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(draw_window(), None));
	    (*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(start_shell(), Some(term)));
	    if ip_str == "10.0.0.1\n" {
	    	//(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(ping_task(net_driver), None));	
	    } else {
	    	//(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(network_task(net_driver), Some(term)));
	    }
	   	(*multitasking::EXECUTOR_PTR).spawn(multitasking::Task::new(keyboard::print_keypresses(), None));
	    (*multitasking::EXECUTOR_PTR).run();
	}
}

async fn start_shell() {
	let fs = unsafe { &mut *fat32::FS_PTR };
	let data = fs.read_file("/SOMNIA").unwrap();	
	fat32::load_elf_and_jump(&data);
}

async fn network_task(mut nic: network::E1000) {
    loop {
        if let Some(_) = nic.recv() {
            println!("Packet recieved!");
        }
        multitasking::cooperate().await
    }
}

async fn ping_task(mut nic: network::E1000) {
    let target = [10,0,0,2];
    loop {
        network::ping(&mut nic, target);
        time::sleep(1000);
        multitasking::cooperate().await
    }
}

async fn draw_window() {
	let gui = unsafe { &mut *gui::GUI_PTR };
	gui.create_window("My window", 450, 50, 200, 150);
}

pub fn hlt_loop() -> ! {
	loop {
		cpu::hlt();
	}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	//vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Red);
	println!("{}", _info);
	hlt_loop();
}

