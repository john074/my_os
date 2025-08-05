#![no_std]

pub mod std;

extern crate alloc;
use crate::std::sysalloc::SysAllocator;

#[global_allocator]
static ALLOC: SysAllocator = SysAllocator;

//pub use std::io::{print, println};

pub use crate::std::syscall::{syscall, exit};
pub use crate::std::time::sleep;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	println!("AAAAAPANIKAA");
    loop {}
}
