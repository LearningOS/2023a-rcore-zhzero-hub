# Building
TARGET := riscv64gc-unknown-none-elf
MODE := release
KERNEL_ELF := target/$(TARGET)/$(MODE)/os
FS_IMG := target/fs.img

# BOARD
BOARD ?= qemu
SBI ?= rustsbi
BOOTLOADER := ../bootloader/$(SBI)-$(BOARD).bin

# fsimg:
# 	cd ../easy-fs-fuse && cargo run --release -- \
# 		-s ../ci-user/user/build/elf \
# 		-o $(FS_IMG)

fs-img: $(APPS)
    make -C ../user build TEST=$(TEST) CHAPTER=$(CHAPTER) BASE=$(BASE)
    rm -f $(FS_IMG)
    cd ../easy-fs-fuse && cargo run --release -- -s ../user/build/elf/ -t ../os/target/

kernel: fsimg
	cargo build --release
clean:
	cargo clean

run: kernel
	timeout --foreground 300s qemu-system-riscv64 \
		-machine virt \
		-nographic \
		-bios $(BOOTLOADER) \
		-kernel $(KERNEL_ELF) \
		-drive file=$(FS_IMG),if=none,format=raw,id=x0 \
		-device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0

.PHONY: build kernel clean run
