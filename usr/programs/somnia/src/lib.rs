#![no_std]

pub mod std;

extern crate alloc;
use crate::std::sysalloc::SysAllocator;

#[global_allocator]
static ALLOC: SysAllocator = SysAllocator;

pub use crate::std::syscall::{syscall, exit};
pub use crate::std::time::sleep;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    use core::fmt::Write;
    println!("Panic: {}", info);
    loop {println!("pan");}
}
