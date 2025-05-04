%define PHYS_OFFSET 0xffff800000000000
%define P4_INDEX 256  ; PHYS_OFFSET >> 39 & 0x1FF = 256
%define P3_INDEX 0
%define P2_INDEX 0

global start
extern long_mode_start

section .text
bits 32
start:
    mov esp, stack_top
    mov edi, ebx ; Multiboot info

    call check_multiboot
    call check_cpuid
    call check_long_mode
    
    call set_up_page_tables
    call enable_paging

    ; load the 64-bit GDT
    lgdt [gdt64.pointer]
    
    jmp 0x08:long_mode_start
    
    hlt


check_multiboot:
    cmp eax, 0x36d76289
    jne .no_multiboot
    ret
.no_multiboot:
    mov al, "0"
    jmp error


check_cpuid:
    ; Check if CPUID is supported by attempting to flip the ID bit (bit 21)
    ; in the FLAGS register. If we can flip it, CPUID is available.

    ; Copy FLAGS in to EAX via stack
    pushfd
    pop eax

    ; Copy to ECX as well for comparing later on
    mov ecx, eax

    ; Flip the ID bit
    xor eax, 1 << 21

    ; Copy EAX to FLAGS via the stack
    push eax
    popfd

    ; Copy FLAGS back to EAX (with the flipped bit if CPUID is supported)
    pushfd
    pop eax

    ; Restore FLAGS from the old version stored in ECX (i.e. flipping the
    ; ID bit back if it was ever flipped).
    push ecx
    popfd

    ; Compare EAX and ECX. If they are equal then that means the bit
    ; wasn't flipped, and CPUID isn't supported.
    cmp eax, ecx
    je .no_cpuid
    ret
.no_cpuid:
    mov al, "1"
    jmp error


check_long_mode:
    ; test if extended processor info in available
    mov eax, 0x80000000    ; implicit argument for cpuid
    cpuid                  ; get highest supported argument
    cmp eax, 0x80000001    ; it needs to be at least 0x80000001
    jb .no_long_mode       ; if it's less, the CPU is too old for long mode

    ; use extended info to test if long mode is available
    mov eax, 0x80000001    ; argument for extended processor info
    cpuid                  ; returns various feature bits in ecx and edx
    test edx, 1 << 29      ; test if the LM-bit is set in the D-register
    jz .no_long_mode       ; If it's not set, there is no long mode
    ret
.no_long_mode:
    mov al, "2"
    jmp error


set_up_page_tables:
    ; Set P4[256] = address of P3 (high-half)
    mov eax, p3_table
    or eax, 0b11
    mov [p4_table + P4_INDEX * 8], eax
    
    ; Set P4[0] = address of P3 (identity)
    mov eax, p3_table
    or eax, 0b11
    mov [p4_table + 0 * 8], eax
    
    ; Set P3[0] = address of P2
    mov eax, p2_table
    or eax, 0b11
    mov [p3_table + 0 * 8], eax
    
    ; Identity + high-half mapping: map 512 * 2MiB = 1 GiB
    mov ecx, 0
.map_loop:
    mov eax, 0x200000        ; 2 MiB
    mul ecx                  ; eax = phys addr
    mov ebx, eax             ; store physical address
    
    or eax, 0b10000011       ; present | writable | huge
    mov [p2_table + ecx * 8], eax
    
    inc ecx
    cmp ecx, 512
    jne .map_loop
    ret
    

enable_paging:
    ; Load page table into CR3
    mov eax, p4_table
    mov cr3, eax

    ; Enable PAE (bit 5) in CR4
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; Enable long mode (bit 8 in EFER MSR)
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; Enable paging (bit 31) in CR0
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax
    ret

; Prints `ERR: ` and the given error code to screen and hangs.
; parameter: error code (in ascii) in al
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

section .bss
align 4096
p4_table:
	resb 4096
p3_table:
    resb 4096
p2_table:
    resb 4096
p3_table_2:   resb 4096
p2_table_2:   resb 4096
p1_table:     resb 4096
stack_bottom:
	 resb 65536
stack_top:

section .rodata
gdt64:
    dq 0                        ; NULL descriptor
.code: equ $ - gdt64
    dq 0x00AF9A000000FFFF       ; Long mode code segment (base=0, limit=0xFFFFF, flags=0x9A)
.pointer:
    dw $ - gdt64 - 1
    dq gdt64
