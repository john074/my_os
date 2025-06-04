#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(ptr_internals)]

use core::panic::PanicInfo;
use core::alloc::{ GlobalAlloc, Layout };
use core::ptr::null_mut;
use core::cell::UnsafeCell;
use core::arch::asm;

mod vga_buffer;
mod interrupts;
mod gdt;
mod carriage;
mod memory;
use crate::memory::FrameAllocator;

#[macro_use]
extern crate bitflags;

#[unsafe(no_mangle)]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
	use x86_64::{ structures::paging::{Translate, Page},  VirtAddr, PhysAddr };
	use x86_64::registers::control::Cr3;
	use multiboot2::{BootInformation, BootInformationHeader};
	use memory::AreaFrameAllocator;

	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Green);
	print!("Hello World!\n>");
	
	interrupts::init_idt();
	gdt::init();
	unsafe { interrupts::PICS.lock().initialize() };
	x86_64::instructions::interrupts::enable();

	let boot_info = unsafe{ BootInformation::load(multiboot_information_address as *const BootInformationHeader).unwrap() };

 	let memory_map_tag = boot_info.memory_map_tag().expect("Memory map tag required");
 	let elf_sections_tag = boot_info.elf_sections().expect("Elf-sections tag required");
	let kernel_start = elf_sections_tag.map(|s| s.start_address()).min().unwrap();
	let elf_sections_tag = boot_info.elf_sections().expect("Elf-sections tag required");
	let kernel_end = elf_sections_tag.map(|s| s.start_address()).max().unwrap();
	println!("Kernel start:{}, kernel end:{}, total:{}", kernel_start, kernel_end, ((kernel_end-kernel_start)/1024)/1024);
	
	let multiboot_start = multiboot_information_address;
	let multiboot_end = multiboot_start + (boot_info.total_size() as usize);
	println!("Multiboot start:{}, multiboot end:{}", multiboot_start, multiboot_end);

	let mut frame_allocator = AreaFrameAllocator::new(kernel_start as usize, kernel_end as usize, multiboot_start, multiboot_end, memory_map_tag.memory_areas());
	enable_nxe_bit();
	enable_write_protect_bit();
	memory::remap_kernel(&mut frame_allocator, &boot_info);
	frame_allocator.allocate_frame();
	println!("NO CRASH! :)");

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if src < dest && (src as usize + n > dest as usize) {
        // copy backwards
        for i in (0..n).rev() {
            *dest.add(i) = *src.add(i);
        }
    } else {
        // copy forwards
        for i in 0..n {
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

pub unsafe fn wrmsr(msr: u32, value: u64) {
	let low = value as u32;
	let high = (value >> 32) as u32;
	unsafe {
		asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nostack, preserves_flags));
	}
}

pub fn rdmsr(msr: u32) -> u64 {
	let low: u32;
	let high: u32;
	unsafe {
		asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr, options(nostack, preserves_flags));
	}
	((high as u64) << 32) | (low as u64)
}

pub unsafe fn cr0_write(val: usize) {
	unsafe {
		asm!("mov cr0, {}", in(reg) val, options(nostack, preserves_flags, nomem));
	}
} 

pub fn cr0() -> usize {
	let val: usize;
	unsafe {
		asm!("mov {}, cr0", out(reg) val, options(nostack, preserves_flags, nomem));
	}
	val
}

struct BumpAllocator {
	heap_start: usize,
	heap_end: usize,
	next: UnsafeCell<usize>,
}

unsafe impl GlobalAlloc for BumpAllocator {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let current = *self.next.get();
		let alloc_start = align_up(current, layout.align());
		let alloc_end = alloc_start.saturating_add(layout.size());

		if alloc_end > self.heap_end {
			null_mut()
		}
		else {
			*self.next.get() = alloc_end;
			alloc_start as *mut u8
		}
	}

	unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
		// to do
	}
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator {
	heap_start: 0x_4444_4444_0000,
	heap_end: 0x_4444_4444_0000 + 100 * 1024,
	next: UnsafeCell::new(0x_4444_4444_0000),	
};

fn align_up(addr: usize, align: usize) -> usize {
	(addr + align - 1) & !(align - 1)
}

fn enable_nxe_bit() {
	const IA32_EFER: u32 = 3221225600;
	let nxe_bit = 1 << 11;
	unsafe {
		let efer = rdmsr(IA32_EFER);
		wrmsr(IA32_EFER, efer | nxe_bit);
	}
}

fn enable_write_protect_bit() {
	const CR0_WRITE_PROTECT: usize = 1 << 16;
	unsafe { cr0_write(cr0() | CR0_WRITE_PROTECT)};
}

unsafe impl Sync for BumpAllocator {}

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

