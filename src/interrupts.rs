use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;

use crate::vga_buffer;
use crate::println;
use crate::gdt;
use crate::keyboard;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> = Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

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

		idt[0x80].set_handler_fn(syscall_interrupt_handler);
		
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

extern "x86-interrupt" fn syscall_interrupt_handler(stack_frame: InterruptStackFrame) {
	let n: u64;
    let a1: u64;
    let a2: u64;
    let a3: u64;

    unsafe {
        core::arch::asm!(
            "mov {}, rax",
            "mov {}, rdi",
            "mov {}, rsi",
            "mov {}, rdx",
            out(reg) n,
            out(reg) a1,
            out(reg) a2,
            out(reg) a3,
        );
    }

    syscall_handler(n, a1, a2, a3);
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    let addr = x86_64::registers::control::Cr2::read();
    panic!("Page Fault at {:#x}, error: {:?}\n{:#?}", addr, error_code, stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Yellow);
	println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
	vga_buffer::WRITER.lock().set_foreground_color(vga_buffer::Color::Green);
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

#[naked]
#[unsafe(no_mangle)]
pub extern "C" fn _syscall_interrupt_handler() {
    unsafe {
        core::arch::naked_asm!(
            "mov r12, rax", // syscall number
            "mov r13, rdi", //arg1
            "mov r14, rsi", //arg2
            "mov r15, rdx", //arg3

            "push r10",
            "push r11",
            "push rbp",
            "push rbx",
            "push r9",
            "push r8",

            "mov rdi, r12",
            "mov rsi, r13",
            "mov rdx, r14",
            "mov rcx, r15",

            "call syscall_handler",

            "pop r8",
            "pop r9",
            "pop rbx",
            "pop rbp",
            "pop r11",
            "pop r10",

            "iretq",
            options()
        );
    }
}

#[unsafe(no_mangle)]
pub fn syscall_handler(number: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
	println!("SYSCALL n={} arg1={:#x} arg2={} arg3={:#x}", number, arg1, arg2, arg3);
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
		_ => {
			println!("Unknown syscall: {}", number);
			ret = -1i64 as u64;
		}
	}

	unsafe {
		core::arch::asm!("mov rax, {}", in(reg) ret, options(preserves_flags));
	}

	ret
}

#[unsafe(no_mangle)]
pub extern "C" fn print_cs_ss(cs: u64, ss: u64) {
    println!(">>> CS: {:#x}, SS: {:#x}", cs, ss);
}

irq_handler!(irq14_handler, 14);
