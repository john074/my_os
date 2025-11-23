// #[unsafe(no_mangle)]
// pub extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
//     for i in 0..n {
//         unsafe { *dest.add(i) = *src.add(i); }
//     }
//     dest
// }

#[cfg(target_arch = "x86_64")]
#[naked]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
	unsafe {
	    core::arch::naked_asm!(
	        "mov rcx, rdx",
	        "rep movsb",
	        "mov rax, rdi",
	        "ret",
	        options()
	    )
	}
}

// #[unsafe(no_mangle)]
// pub extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
//     for i in 0..n {
//         unsafe { *s.add(i) = c as u8; }
//     }
//     s
// }

#[cfg(target_arch = "x86_64")]
#[naked]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    unsafe {
        core::arch::naked_asm!(
            "mov rcx, rdx",    // n -> rcx (count register)
            "mov al, sil",     // c -> al (lower 8 bits of rsi)
            "mov rdi, rdi",    // dest -> rdi (destination)
            "rep stosb",       // repeat store byte
            "mov rax, rdi",    // return original dest
            "ret",
            options()
        )
    }
}

// #[unsafe(no_mangle)]
// pub extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
//     for i in 0..n {
//         let a = unsafe { *s1.add(i) };
//         let b = unsafe { *s2.add(i) };
//         if a != b {
//             return a as i32 - b as i32;
//         }
//     }
//     0
// }

// 3 = 1
#[cfg(target_arch = "x86_64")]
#[naked]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    unsafe {
        core::arch::naked_asm!(
            "mov rcx, rdx",    // n -> rcx
            "xor rax, rax",    // clear rax for return value
            "repe cmpsb",      // repeat compare bytes
            "je 3f",           // if equal, return 0
            "movzx eax, byte ptr [rdi - 1]",  // get differing byte from s1
            "movzx ecx, byte ptr [rsi - 1]",  // get differing byte from s2  
            "sub eax, ecx",    // calculate difference
            "3:",
            "ret",
            options()
        )
    }
}

// #[unsafe(no_mangle)]
// pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
//     if src < dest && (src as usize + n > dest as usize) {
//         // copy backwards
//         for i in (0..n).rev() {
//             *dest.add(i) = *src.add(i);
//         }
//     } else {
//         // copy forwards
//         for i in 0..n {
//             *dest.add(i) = *src.add(i);
//         }
//     }
//     dest
// }

// 3 = 1
#[cfg(target_arch = "x86_64")]
#[naked]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    unsafe {
        core::arch::naked_asm!(
            "test rdx, rdx",   // check if n == 0
            "jz 2f",           // if zero, return immediately
            
            "mov rcx, rdx",    // n -> rcx
            "cmp rsi, rdi",    // compare src and dest
            "jae 3f",          // if src >= dest, copy forward
            
            // Backward copy
            "lea rsi, [rsi + rcx - 1]",  
            "lea rdi, [rdi + rcx - 1]",  
            "std",                       
            "rep movsb",
            "cld",                       
            "jmp 2f",
            
            // Forward copy  
            "3:",
            "rep movsb",
            
            // Return
            "2:",
            "mov rax, rdi",    
            "ret",
            options()
        )
    }
}


#[unsafe(no_mangle)]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}
