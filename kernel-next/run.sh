#!/bin/bash

set -ex

SOURCE_DIR=$(pwd)

mkdir -p esp/efi/boot

cd ../bootloader
    cargo build --release
cd ../kernel-next
    # cargo build --release
    cargo rustc \
		--release \
		--manifest-path "$SOURCE_DIR/Cargo.toml" \
		--target "$SOURCE_DIR/x86_64-kernel.json" \
		-Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem \
		-- \
		-C link-arg=-T -Clink-arg="$SOURCE_DIR/kernel_x86_64.ld" \
		-C link-arg=-z -Clink-arg=max-page-size=0x1000 \
		--emit link="$SOURCE_DIR/esp/kernel"

    cp ../target/x86_64-unknown-uefi/release/bootloader.efi esp/efi/boot/bootx64.efi

qemu-system-x86_64 \
    -accel kvm \
    -m 256M \
    -smp 4 \
    -rtc base=utc \
    -display gtk,show-tabs=on \
    -drive if=pflash,format=raw,readonly=on,file=firmware/uefi/OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=firmware/uefi/OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -device virtio-keyboard \
    -device virtio-mouse \
    -vga virtio \
    -serial stdio
