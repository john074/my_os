use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;

use alloc::boxed::Box;

use crate::println;
use crate::debug;
use crate::gdt;
use crate::keyboard;
use crate::fs;
use crate::multitasking;
use crate::memory;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

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
	println!("Interrupts initialization\t[OK]");
}

lazy_static! {
	static ref IRQ_HANDLERS: Mutex<[fn(); 16]> = {
		Mutex::new([default_handler; 16])
	};
	
	static ref IDT: InterruptDescriptorTable = {
		let mut idt = InterruptDescriptorTable::new();
		idt.breakpoint.set_handler_fn(breakpoint_handler);
		idt.page_fault.set_handler_fn(page_fault_handler);
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

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
	debug!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
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
    let _arg4 = regs.r8 as u64;

    debug!("SYSCALL n={} arg1={:#x} arg2={} arg3={:#x}", n, arg1, arg2, arg3);
    
    let res = _syscall_handler(n, arg1, arg2, arg3) as usize;

    regs.rax = res;

    unsafe { PICS.lock().notify_end_of_interrupt(0x80) };
}

// extern "x86-interrupt" fn syscall_interrupt_handler(stack_frame: InterruptStackFrame) {
// 	let n: u64;
//     let a1: u64;
//     let a2: u64;
//     let a3: u64;
// 
//     unsafe {
//         core::arch::asm!(
//             "mov r12, rax", // syscall number
//             "mov r13, rdi",
//             "mov r14, rsi",
//             "mov r15, rdx",
// 
//             "mov {0}, r12",
//             "mov {1}, r13",
//             "mov {2}, r14",
//             "mov {3}, r15",
//             out(reg) n,
//             out(reg) a1,
//             out(reg) a2,
//             out(reg) a3,
//         );
//     }
//     
//     syscall_handler(n, a1, a2, a3);
// }

#[unsafe(no_mangle)]
pub fn _syscall_handler(number: u64, arg1: u64, arg2: u64, arg3: u64) -> u64  {
	let ret;
	match number {
		1 => { // SYS_WRITE
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::slice::from_raw_parts(ptr, len) };
			if let Ok(text) = core::str::from_utf8(s) {
				println!("{}", text);
			}
			ret = 0;
		}
		2 => { // SYS_EXIT
		    debug!("Program requested exit via syscall.");
		    unsafe {
		        let addr = fs::RETURN_ADDR;
		        core::arch::asm!("jmp {}", in(reg) addr, options(noreturn));
		    }
		}
		3 => { // SYS_
			ret = 0;
		}
		4 => { // SYS_
			ret = 0;
		}
		5 => {
    		unsafe {
		        let raw_ptr = arg1 as *mut multitasking::Task;
		        let boxed = Box::from_raw(raw_ptr);
		        fs::EXECUTOR_PTR.as_mut().unwrap().spawn(*boxed);
		        fs::EXECUTOR_PTR.as_mut().unwrap().run();
		    }
		}		
		6 => { // SYS_ALLOC
				use alloc::alloc::{ GlobalAlloc, Layout };
				let layout = Layout::from_size_align(arg1 as usize, arg2 as usize).unwrap();
			  	let ptr = unsafe {
			        memory::ALLOCATOR.alloc(layout)
			    };
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
		8 => { // SYS_WRITE
			println!("{}", arg1);
			ret = 0;
		}		
		_ => {
			debug!("Unknown syscall: {}", number);
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
    debug!(">>> CS: {:#x}, SS: {:#x}", cs, ss);
}

irq_handler!(irq14_handler, 14);
