#![no_std]
#![no_main]

mod multitasking;
mod std;

extern crate alloc;
use core::panic::PanicInfo;

#[repr(u64)]
pub enum SyscallNumber {
    Write = 1,
    Exit = 2,
    SpawnTask = 5,
    Alloc = 6,
    Dealloc = 7,
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
	let mut task = multitasking::Task::new(user());
	syscall(SyscallNumber::SpawnTask as u64, (&mut task as *mut multitasking::Task) as u64, 0, 0);
    exit();
}

async fn user() {
	let txt = "YOO, hello from programm!!!";
	print(txt);
	loop {
	    //print("working...\n");
	   	multitasking::cooperate().await;
		//sleep(2000);
	}
	exit();	
}

pub fn syscall(n: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("rax") n,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") ret,
            options(nostack)
        );
    }
    ret
}


pub fn print(s: &str) {
    syscall(SyscallNumber::Write as u64, s.as_ptr() as u64, s.len() as u64, 0);
}

pub fn print_u64(s: u64) {
    syscall(8, s, 0, 0);
}

pub fn exit() {
	syscall(SyscallNumber::Exit as u64, 0, 0, 0);
}

pub fn syscall_alloc(size: u64, align: u64) -> u64 {
	syscall(SyscallNumber::Alloc as u64, size, align, 0)
}

pub fn syscall_dealloc(ptr: u64, size: u64, align: u64) {
	syscall(SyscallNumber::Dealloc as u64, ptr, size, align);
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop{}
}

pub fn sleep(ms: u64) {
	let cycles = ms / 1000 * 3_000_000;
	unsafe {
		for _ in 0..cycles {
			core::arch::asm!("pause");
		}
	}
}


use core::alloc::{GlobalAlloc, Layout};

struct SysAllocator;

unsafe impl GlobalAlloc for SysAllocator {
   unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        syscall_alloc(layout.size() as u64, layout.align() as u64) as *mut u8
    }
    

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        syscall_dealloc(ptr as u64, layout.size() as u64, layout.align() as u64)
    }
}

#[global_allocator]
static ALLOC: SysAllocator = SysAllocator;
