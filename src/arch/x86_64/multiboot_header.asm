section .multiboot_header
align 8
header_start:
    dd 0xe85250d6                ; magic number (multiboot 2)
    dd 0                         ; architecture = i386
    dd header_end - header_start ; header length
    ; checksum
    dd 0x100000000 - (0xe85250d6 + 0 + (header_end - header_start))

    ; --- Framebuffer request tag (type=5) ---
    align 8
    dw 5        ; type
    dw 0        ; flags
    dd 24       ; size (must be multiple of 8)
    dd 1024     ; width
    dd 768      ; height
    dd 32       ; depth

    ; --- End tag (type=0, size=8) ---
    align 8
    dw 0 	; type
    dw 0 	; flags
    dd 8 	; size

header_end:
