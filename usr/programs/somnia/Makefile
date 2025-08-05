# Makefile

TARGET = x86_64-my_os
BUILD_DIR = target/$(TARGET)/debug

all: build

build:
	RUSTFLAGS="-C link-arg=-Ttext=0x444444640000 -C link-arg=--nmagic" \
	    cargo +nightly build --release -Z build-std=core,alloc --target=$(TARGET).json
	dd if=/dev/zero of=../fat32.img bs=512 count=288000
	mkfs.vfat -F 32 ../fat32.img
	mcopy -i ../fat32.img target/x86_64-my_os/release/somnia ::
	#cp -f ../test_program/fat32.img ../my_os/fat32.img
	  
clean:
	cargo clean
	rm -f $(KERNEL) $(ISO_IMAGE)

.PHONY: all build link iso run clean
