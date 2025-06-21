# Makefile

TARGET = x86_64-my_os
BUILD_DIR = target/$(TARGET)/debug
KERNEL = kernel.bin
ISO_DIR = iso/boot
ISO_IMAGE = boot.iso

all: build assembler link iso run

build:
	cargo +nightly build -Z build-std=core,alloc,compiler_builtins --target=$(TARGET).json
assembler:
	nasm -felf64 src/arch/x86_64/boot.asm -o boot.o
	nasm -felf64 src/arch/x86_64/long_mode_init.asm -o long_mode_init.o
	nasm -felf64 src/arch/x86_64/multiboot_header.asm -o multiboot_header.o

link:
	ld -n -T src/arch/x86_64/linker.ld -o $(KERNEL) boot.o long_mode_init.o multiboot_header.o $(BUILD_DIR)/libmy_os.a

iso:
	cp -f $(KERNEL) $(ISO_DIR)/
	grub-mkrescue -o $(ISO_IMAGE) iso

run:
	qemu-system-x86_64 \
  -drive file=boot.iso,format=raw,media=cdrom \
  -drive file=fat12.img,format=raw,if=ide,index=1,media=disk \
  -boot order=d \
  -serial stdio \
  -machine pc \
  #-D qemu.log -d int,cpu,exec \
	    
	  
	
clean:
	cargo clean
	rm -f $(KERNEL) $(ISO_IMAGE)

.PHONY: all build link iso run clean
