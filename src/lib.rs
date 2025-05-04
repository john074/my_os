#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

mod vga_buffer;
mod interrupts;
mod gdt;
mod carriage;
mod memory;


#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
	use x86_64::{ structures::paging::{Translate, Page},  VirtAddr, PhysAddr };
	use x86_64::registers::control::Cr3;
	use memory::BootInfoFrameAllocator;

	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Green);
	print!("Hello World!\n>");
	
	interrupts::init_idt();
	gdt::init();
	unsafe { interrupts::PICS.lock().initialize() };
	x86_64::instructions::interrupts::enable();

	let phys_mem_offset = VirtAddr::new(0xffff800000000000);
	let mut mapper = unsafe { memory::init(phys_mem_offset) };
// 	let mut frame_allocator = unsafe {
// 		BootInfoFrameAllocator::init()
// 	};
// 
// 	let page = Page::containing_address(VirtAddr::new(0x_ffff_8000_4000_0001));
// 	memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);
// 
// 	let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
// 	unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e)};

// 	let phys_mem_offset = VirtAddr::new(0xffff800000000000);
// 	let mapper = unsafe { memory::init(phys_mem_offset) };
// 
// 	let addresses = [
// 		// the identity-mapped vga buffer page
// 	    0xb8000,
// 	    // some code page
// 	    0x201008,
// 	    // some stack page
// 	    0x0,
// 	    // virtual address mapped to physical address 0
// 	    0xffff800000000000,
// 	    0xffff800000abcdef,
// 	    0x10000201a10,
// 	];
// 	
// 	for &address in &addresses {
// 	    let virt = VirtAddr::new(address);
// 	    // new: use the `mapper.translate_addr` method
// 	    let phys = mapper.translate_addr(virt);
// 	    println!("{:?} -> {:?}", virt, phys);
// 	}
	
	hlt_loop();
	
}

#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe { *dest.add(i) = *src.add(i); }
    }
    dest
}

#[unsafe(no_mangle)]
pub extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    for i in 0..n {
        unsafe { *s.add(i) = c as u8; }
    }
    s
}

#[unsafe(no_mangle)]
pub extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let a = unsafe { *s1.add(i) };
        let b = unsafe { *s2.add(i) };
        if a != b {
            return a as i32 - b as i32;
        }
    }
    0
}

pub fn hlt_loop() -> ! {
	loop {
		x86_64::instructions::hlt();
	}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Red);
	println!("{}", _info);
	hlt_loop();
}

