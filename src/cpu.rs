use core::arch::asm;

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

pub unsafe fn write_raw_cr3(address: u64) {
	unsafe {
		asm!("mov cr3, {}", in(reg) address, options(nostack, nomem, preserves_flags));
	}
}

pub fn cr3() -> u64 {
	let val: u64;
	unsafe {
		asm!("mov {}, cr3", out(reg) val, options(nostack, preserves_flags, nomem));
	}
	val
}

pub fn rdtsc() -> u64 {
	let low: u32;
	let high: u32;

	unsafe {
		asm!("rdtsc", out("eax") low, out("edx") high, options(nomem, nostack, preserves_flags));
	}
	((high as u64) << 32) | (low as u64)
}

pub fn hlt() {
	unsafe {
		asm!("hlt", options(nomem, nostack, preserves_flags));		
	}
}

pub fn enable_interrupts() {
	unsafe {
		asm!("sti", options(nomem, nostack));
	}
}

pub fn disable_interrupts() {
	unsafe {
		asm!("cli", options(nomem, nostack));
	}
}

pub fn set_iopl() {
	unsafe {
	    let mut flags: u64;
	    asm!(
	        "pushfq",
	        "pop {}",
	        out(reg) flags,
	    );
	    flags |= 0b11 << 12; // IOPL = 3
	    asm!(
	        "push {}",
	        "popfq",
	        in(reg) flags,
	    );
	}
}

use crate::println;
pub fn check_cpl() {
    unsafe {
        let cs: u16;
        asm!("mov {}, cs", out(reg) cs);
        println!("Current privilege level: {}", cs & 0b11);
    }
}

pub fn enable_nxe_bit() {
	const IA32_EFER: u32 = 3221225600;
	let nxe_bit = 1 << 11;
	unsafe {
		let efer = rdmsr(IA32_EFER);
		wrmsr(IA32_EFER, efer | nxe_bit);
	}
}

pub fn enable_write_protect_bit() {
	const CR0_WRITE_PROTECT: usize = 1 << 16;
	unsafe { cr0_write(cr0() | CR0_WRITE_PROTECT)};
}
