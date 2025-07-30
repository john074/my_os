use core::alloc::{GlobalAlloc, Layout};
use crate::std::syscall::{syscall_alloc, syscall_dealloc};

pub struct SysAllocator;

unsafe impl GlobalAlloc for SysAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        syscall_alloc(layout.size() as u64, layout.align() as u64) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        syscall_dealloc(ptr as u64, layout.size() as u64, layout.align() as u64)
    }
}
