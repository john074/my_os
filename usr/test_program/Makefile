# Makefile

TARGET = x86_64-my_os
BUILD_DIR = target/$(TARGET)/debug

all: build

build:
	cargo +nightly build --release -Z build-std=core,alloc --target=$(TARGET).json
	dd if=/dev/zero of=fat12.img bs=512 count=2880
	mkfs.vfat -F 12 fat12.img
	mcopy -i fat12.img target/x86_64-my_os/release/APP ::
	#cp -f fat12.img ../my_os/fat12.img
	#objcopy -O binary target/x86_64-my_os/release/APP APP.bin
	  
clean:
	cargo clean
	rm -f $(KERNEL) $(ISO_IMAGE)

.PHONY: all build link iso run clean
