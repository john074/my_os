ENTRY(start)

PHYS_OFFSET = 0xffff800000000000;

SECTIONS {
    . = 1M;

    .boot :
    {
        KEEP(*(.multiboot2_header))
    }

    .text :
    {
        *(.text)
    }

    .rodata :
    {
        *(.rodata)
    }

    .bss :
    {
        *(.bss COMMON)
    }

    /DISCARD/ : { *(.eh_frame) }
}
