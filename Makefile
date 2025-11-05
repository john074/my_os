TARGET = x86_64-my_os
BUILD_DIR = target/$(TARGET)/debug
KERNEL = kernel.bin
ISO_DIR = iso/boot
ISO_IMAGE = boot.iso

PROGRAM_DIRS := $(shell find usr/programs -mindepth 1 -maxdepth 1 -type d)
STD_DIR := usr/std

all: usr build assembler link iso run

usr:
	dd if=/dev/zero of=usr/programs/fat32.img bs=512 count=288000
	mkfs.vfat -F 32 usr/programs/fat32.img

	@for dir in $(PROGRAM_DIRS); do \
		echo "Copying std to $$dir/src/"; \
		cp -r $(STD_DIR) $$dir/src/ || exit 1; \
		if [ -f $$dir/Makefile ]; then \
			echo "Building $$dir"; \
			$(MAKE) -C $$dir all || exit 1; \
		fi \
	done
	mv usr/programs/fat32.img .

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
  -vga std \
  -serial stdio \
  -machine pc \
  -s
  #-D qemu.log -d int,cpu,exec \
	    
clean:
	cargo clean
	rm -f $(KERNEL) $(ISO_IMAGE)

.PHONY: all usr build link iso run clean
