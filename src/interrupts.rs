use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use alloc::vec::Vec;
use spin::Mutex;

use alloc::boxed::Box;

use crate::println;
use crate::print;
use crate::gdt;
use crate::keyboard;
use crate::fat32;
use crate::multitasking;
use crate::memory;
use crate::vga_buffer;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

static mut SCANCODE_STREAM: Option<keyboard::ScancodeStream> = None; 

#[repr(align(8), C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Registers {
    // Saved scratch registers
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rdi: usize,
    pub rsi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: usize,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
	Timer = PIC_1_OFFSET,
	Keyboard,
}

impl InterruptIndex {
	fn as_u8(self) -> u8 {
		self as u8
	}

	fn as_usize(self) -> usize {
		usize::from(self.as_u8())
	}
}

pub fn init() {
	IDT.load();
	println!("Interrupt descriptor table is set.");
	gdt::init();
	println!("Global descriptor table is initialized");
	unsafe { PICS.lock().initialize(); }
	println!("Programmable interrupt controller is initialized.");
	x86_64::instructions::interrupts::enable();
	println!("Interrupts initialization    [OK]");
}

lazy_static! {
	static ref IRQ_HANDLERS: Mutex<[fn(); 16]> = {
		Mutex::new([default_handler; 16])
	};
	
	static ref IDT: InterruptDescriptorTable = {
		let mut idt = InterruptDescriptorTable::new();
		idt.breakpoint.set_handler_fn(breakpoint_handler);
		idt.page_fault.set_handler_fn(page_fault_handler);
		
		idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
		idt.segment_not_present.set_handler_fn(segment_not_present_handler);
		unsafe {
			idt.double_fault.set_handler_fn(double_fault_handler).set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
			idt.general_protection_fault.set_handler_fn(general_protection_fault_handler).set_stack_index(gdt::GENERAL_PROTECTION_FAULT_IST_INDEX);
		}
		idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
		idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);

		idt[(PIC_1_OFFSET + 14) as usize].set_handler_fn(irq14_handler);
		
		//idt[0x80].set_handler_fn(syscall_interrupt_handler);
		unsafe {
			let f = wrapped_syscall_handler as *mut fn();
			idt[0x80].set_handler_fn(core::mem::transmute(f)).set_privilege_level(x86_64::PrivilegeLevel::Ring3);
		}
		
		idt
	};		
}

fn default_handler() {}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
	unsafe {
		PICS.lock().notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
	}
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
	use x86_64::instructions::port::Port;
	
	let mut port = Port::new(0x60);
	let scancode: u8 = unsafe { port.read() };
	keyboard::add_scancode(scancode);

	unsafe {
		PICS.lock().notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
	}
}

extern "x86-interrupt" fn general_protection_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) {
    panic!("EXCEPTION: GENERAL PROTECTION FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
	panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    let addr = x86_64::registers::control::Cr2::read();
    panic!("Page Fault at {:#x}, error: {:?}\n{:#?}", addr, error_code, stack_frame);
}

extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: STACK SEGMENT FAULT\nStack Frame: {:#?}\nError: {:?}", stack_frame, error_code);
}

extern "x86-interrupt" fn segment_not_present_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!("EXCEPTION: SEGMENT NOT PRESENT\nStack Frame: {:#?}\nError: {:?}", stack_frame, error_code);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
	println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

macro_rules! irq_handler {
    ($handler:ident, $irq:expr) => {
        pub extern "x86-interrupt" fn $handler(_: InterruptStackFrame) {
            let handlers = IRQ_HANDLERS.lock();
            handlers[$irq]();
            unsafe {
                PICS.lock().notify_end_of_interrupt(
                    (PIC_1_OFFSET + $irq) as u8
                );
            }
        }
    };
}

macro_rules! wrap {
    ($fn: ident => $w:ident) => {
        #[naked]
        pub unsafe extern "sysv64" fn $w() {
           core::arch:: naked_asm!(
                "push rax",
                "push rcx",
                "push rdx",
                "push rsi",
                "push rdi",
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "mov rsi, rsp", // Arg #2: register list
                "mov rdi, rsp", // Arg #1: interupt frame
                "add rdi, 9 * 8", // 9 registers * 8 bytes
                "call {}",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rdi",
                "pop rsi",
                "pop rdx",
                "pop rcx",
                "pop rax",
                "iretq",
                sym $fn
            );
        }
    };
}

wrap!(syscall_handler => wrapped_syscall_handler);

extern "sysv64" fn syscall_handler(_stack_frame: &mut InterruptStackFrame, regs: &mut Registers) {
    let n = regs.rax as u64; 
    // The registers order follow the System V ABI convention
    let arg1 = regs.rdi as u64;
    let arg2 = regs.rsi as u64;
    let arg3 = regs.rdx as u64;
    let arg4 = regs.r8 as u64;

	if n != 3 && n != 1 {
    	//println!("SYSCALL n={} arg1={:#x} arg2={} arg3={:#x}", n, arg1, arg2, arg3);
    }
    
    let res = _syscall_handler(n, arg1, arg2, arg3, arg4) as usize;

    regs.rax = res;

    unsafe { PICS.lock().notify_end_of_interrupt(0x80) };
}

#[unsafe(no_mangle)]
pub fn _syscall_handler(number: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64) -> u64  {
	let mut ret;
	match number {
		1 => { // SYS_WRITE
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				print!("{}", text);
				// if text.contains("\n") {
				// 	print!(">");
				// }
			}
			ret = 0;
		}
		2 => { // SYS_EXIT
		    println!("Program requested exit via syscall.");
		    unsafe {
		        let addr = fat32::RETURN_ADDR;
		        core::arch::asm!("jmp {}", in(reg) addr, options(noreturn));
		        ret = 0;
		    }
		}
		3 => { // SYS_READ
			let user_buffer = unsafe { &mut *(arg1 as *mut Vec<char>) };
			let data: Vec<char> = keyboard::INPUT_BUFFER.lock().iter().cloned().collect();
			keyboard::INPUT_BUFFER.lock().clear();
			for i in data {
				user_buffer.push(i);
			}
			ret = 0;		    
		}
		4 => { // SYS_RM_CHAR
			vga_buffer::WRITER.lock().rm_char();
			ret = 0;
		}
		5 => {
    		unsafe {
		        let raw_ptr = arg1 as *mut multitasking::Task;
		        //(*raw_ptr).id = multitasking::TaskId::new();
		        let boxed = Box::from_raw(raw_ptr);
		        fat32::EXECUTOR_PTR.as_mut().unwrap().spawn(*boxed);
		        fat32::EXECUTOR_PTR.as_mut().unwrap().run();
		    }
		    ret = 0;
		}		
		6 => { // SYS_ALLOC
				use alloc::alloc::{ GlobalAlloc, Layout };
				let layout = Layout::from_size_align(arg1 as usize, arg2 as usize).unwrap();
			  	let ptr = unsafe {
			        memory::ALLOCATOR.alloc(layout)
			    };
			    //println!("ALLOC returns: 0x{:x}", ptr as u64);
			    ret = ptr as u64;
		}
		7 => { // SYS_DEALLOC
				use alloc::alloc::{ GlobalAlloc, Layout };
				let layout = Layout::from_size_align(arg2 as usize, arg3 as usize).unwrap();
				unsafe {
			    	memory::ALLOCATOR.dealloc(arg1 as *mut u8, layout);
			    }
			    ret = 0;
		}
		8 => { // SYS_CHECK_DIR_EXISTS
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			ret = 0;
			if let Ok(text) = core::str::from_utf8(s) {
				let fs = unsafe { &mut *fat32::FS_PTR };
				if fs.find_path(text).is_some() {
					ret = 1;
				}
			}
		}	
		9 => { // SYS_GEN_TASK_ID
			ret = multitasking::TaskId::new().0;
		}
		10 => { // SYS_LIST_DIR
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				let fs = unsafe { &mut *fat32::FS_PTR };
				let data = fs.list_dir(text).unwrap();
				let user_buffer = unsafe { &mut *(arg3 as *mut Vec<alloc::string::String>) };
				for i in data {
					user_buffer.push(i);
				}
			}
			ret = 0;
		}	
		11 => { // SYS_MKDIR
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				let fs = unsafe { &mut *fat32::FS_PTR };
				fs.create_directory(text);
			}
			ret = 0;
		}
		12 => { // SYS_MKFILE
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				let fs = unsafe { &mut *fat32::FS_PTR };
				fs.create_file(text, 0);
			}
			ret = 0;
		}
		13 => { // SYS_WRITE_FILE
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				let content_ptr = arg3 as *const u8;
				let content_len = arg4 as usize;
				let data = unsafe { core::slice::from_raw_parts(content_ptr, content_len) };
				let fs = unsafe { &mut *fat32::FS_PTR };
				fs.write_file(text, data);
			}
			ret = 0;
		},
		14 => { // SYS_READ_FILE
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				let fs = unsafe { &mut *fat32::FS_PTR };
				let user_ptr = arg3 as *mut u8;
				let user_len = arg4 as usize;
				let bytes = fs.read_file(text).unwrap();
				let copy_len = core::cmp::min(user_len, bytes.len());
				unsafe {
			    	core::ptr::copy_nonoverlapping(bytes.as_ptr(), user_ptr, copy_len);
				}
				ret = copy_len as u64;
			}
			else {
				ret = 0;
			}
		},
		15 => { // SYS_MKFILE
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				let fs = unsafe { &mut *fat32::FS_PTR };
				fs.delete_directory(text);
			}
			ret = 0;
		}
		16 => { // SYS_MKFILE
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				let fs = unsafe { &mut *fat32::FS_PTR };
				fs.delete_file(text);
			}
			ret = 0;
		}		
		_ => {
			println!("Unknown syscall: {}", number);
			ret = -1i64 as u64;
		}
	}

	unsafe {
		core::arch::asm!("mov rax, {0}", in(reg) ret, options(nostack, preserves_flags));
	}

	ret
}

#[unsafe(no_mangle)]
pub extern "C" fn print_cs_ss(cs: u64, ss: u64) {
    println!(">>> CS: {:#x}, SS: {:#x}", cs, ss);
}

irq_handler!(irq14_handler, 14);
