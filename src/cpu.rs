use core::arch::asm;
//use crate::gdt;

// const IA32_EFER: u32 = 0xC0000080;
// const IA32_STAR: u32 = 0xC0000081;
// const IA32_LSTAR: u32 = 0xC0000082;
// const IA32_FMASK: u32 = 0xC0000084;

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

pub fn check_cpl() -> u16 {
    unsafe {
        let cs: u16;
        asm!("mov {}, cs", out(reg) cs);
        cs
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

// pub unsafe fn init_syscall(syscall_entry: u64) {
//     // Turn on SCE bit (System Call Extensions) in EFER
//     let mut efer = rdmsr(IA32_EFER);
//     efer |= 1; // EFER.SCE = 1
//     wrmsr(IA32_EFER, efer);
// 
//     let user_cs: u64 = gdt::GDT.1.user_code_selector.0 as u64;
//     let kernel_cs: u64 = gdt::GDT.1.code_selector.0 as u64;
//     let star = (user_cs << 48) | (kernel_cs << 32);
//     wrmsr(IA32_STAR, star);
// 
//     // Set RFLAGS mask
//     let rflags_mask: u64 = 1 << 9; // IF
//     wrmsr(IA32_FMASK, rflags_mask);
// 
//     // Set syscall handler address
//     wrmsr(IA32_LSTAR, syscall_entry);
// }

