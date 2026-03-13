TARGET = x86_64-my_os
BUILD_DIR = target/$(TARGET)/debug
KERNEL = kernel.bin
ISO_DIR = iso/boot
ISO_IMAGE = boot.iso

PROGRAM_DIRS := $(shell find usr/programs -mindepth 1 -maxdepth 1 -type d)
STD_DIR := usr/std

all: usr build assembler link iso ip run

net_all: usr build assembler link iso net_ip setup_interfaces net_run run

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

ip:
	@echo "10.0.0.1" > ip.txt
	sudo mount -o loop fat32.img /mnt
	sudo cp ip.txt /mnt/
	sudo umount /mnt
	rm ip.txt

net_ip:
	@cp fat32.img fat32_copy.img
	@echo "10.0.0.1" > ip.txt
	sudo mount -o loop fat32.img /mnt
	sudo cp ip.txt /mnt/
	sudo umount /mnt
	@echo "10.0.0.2" > ip.txt
	sudo mount -o loop fat32_copy.img /mnt
	sudo cp ip.txt /mnt/
	sudo umount /mnt
	rm ip.txt

setup_interfaces:
	@echo "Setting up network interfaces"
	@if ! ip link show tap0 >/dev/null 2>&1; then \
		echo "Adding tap0"; \
		sudo ip tuntap add tap0 mode tap; \
	else \
		echo "tap0 already exists"; \
	fi
	@if ! ip link show tap1 >/dev/null 2>&1; then \
		echo "Adding tap1"; \
		sudo ip tuntap add tap1 mode tap; \
	else \
		echo "tap1 already exists"; \
	fi
	@if ! ip link show br0 >/dev/null 2>&1; then \
		echo "Adding bridge br0"; \
		sudo ip link add br0 type bridge; \
	else \
		echo "br0 already exists"; \
	fi
	@echo "Setting interfases state to UP"
	sudo ip link set tap0 up 2>/dev/null || echo "tap0 is already UP"
	sudo ip link set tap1 up 2>/dev/null || echo "tap1 is already UP"
	@echo "Adding interfaces to bridge"
	@if ! sudo ip link show | grep -q "tap0@br0"; then \
		sudo ip link set tap0 master br0 2>/dev/null || echo "tap0 is already in  bridge"; \
	else \
		echo "tap0 is alreadt in bridge bridge"; \
	fi
	@if ! sudo ip link show | grep -q "tap1@br0"; then \
		sudo ip link set tap1 master br0 2>/dev/null || echo "tap1 is already in bridge"; \
	else \
		echo "tap1 is already in bridge"; \
	fi
	sudo ip link set br0 up 2>/dev/null || echo "br0 is already UP"

run:
	qemu-system-x86_64 \
		-drive file=boot.iso,format=raw,media=cdrom \
		-drive file=fat32.img,format=raw,if=ide,index=1,media=disk \
		-boot order=d \
		-vga std \
		-serial stdio \
		-machine pc \
		-device e1000,netdev=n1,mac=52:54:00:12:34:01 \
		-netdev tap,id=n1,ifname=tap0,script=no,downscript=no &
		#-D qemu.log -d int,cpu,exec \

net_run:
	qemu-system-x86_64 \
		-drive file=boot.iso,format=raw,media=cdrom \
		-drive file=fat32_copy.img,format=raw,if=ide,index=1,media=disk \
		-boot order=d \
		-vga std \
		-serial stdio \
		-machine pc \
		-device e1000,netdev=n1,mac=52:54:00:12:34:02 \
		-netdev tap,id=n1,ifname=tap1,script=no,downscript=no &
		#-D qemu.log -d int,cpu,exec \

clean:
	cargo clean
	rm -f $(KERNEL) $(ISO_IMAGE) fat32_copy.img

.PHONY: all net_all usr build assembler link iso drive ip net_ip setup_interfaces run net_run clean
