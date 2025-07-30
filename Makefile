# Makefile

TARGET = x86_64-my_os
BUILD_DIR = target/$(TARGET)/debug
KERNEL = kernel.bin
ISO_DIR = iso/boot
ISO_IMAGE = boot.iso

USR_DIRS := $(shell find usr -mindepth 1 -maxdepth 1 -type d)

all: usr build assembler link iso run

usr:
	@for dir in $(USR_DIRS); do \
		if [ -f $$dir/Makefile ]; then \
			$(MAKE) -C $$dir all; \
		fi \
	done
	mv usr/fat32.img .

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

drive:
	dd if=/dev/zero of=fat32.img bs=512 count=288000
	mkfs.vfat -F 32 fat32.img

run:
	qemu-system-x86_64 \
  -drive file=boot.iso,format=raw,media=cdrom \
  -drive file=fat32.img,format=raw,if=ide,index=1,media=disk \
  -boot order=d \
  -serial stdio \
  -machine pc \
  #-D qemu.log -d int,cpu,exec \
	    
	  
	
clean:
	cargo clean
	rm -f $(KERNEL) $(ISO_IMAGE)

.PHONY: all usr build link iso run clean
