#!/bin/bash

# FIXME: This only works on Linux-based systems.

set -e # Exit on error.

SOURCE_DIR=$(pwd)

# Create the EFI System Partition (ESP) directory.
mkdir -p esp/efi/boot

set -e

# Ensure OVMF (Open Virtual Machine Firmware) is available, and in the correct location.
#
# If you get an error here, make sure you have the OVMF package installed (something along the
# lines of `sudo apt install ovmf`, depends on your distribution).
mkdir -p firmware/uefi
if [[ ! -e "firmware/uefi/OVMF_CODE.fd" ]]; then
    echo "File 'firmware/uefi/OVMF_CODE.fd' does not exist, copying from system..."
    cp /usr/share/OVMF/OVMF_CODE.fd firmware/uefi/OVMF_CODE.fd
fi
if [[ ! -e "firmware/uefi/OVMF_VARS.fd" ]]; then
    echo "File 'firmware/uefi/OVMF_VARS.fd' does not exist, copying from system..."
    cp /usr/share/OVMF/OVMF_VARS.fd firmware/uefi/OVMF_VARS.fd
fi

# Build the bootloader.
cd ../bootloader
    cargo build --release

# Build the kernel.
cd ../kernel
    cargo rustc \
		--release \
		--manifest-path "$SOURCE_DIR/Cargo.toml" \
		--target "$SOURCE_DIR/x86_64-kernel.json" \
		-Z build-std=core,alloc -Zbuild-std-features=compiler-builtins-mem \
		-- \
		-C link-arg=-T -Clink-arg="$SOURCE_DIR/kernel_x86_64.ld" \
		-C link-arg=-z -Clink-arg=max-page-size=0x1000 \
		--emit link="$SOURCE_DIR/esp/kernel"

    # Place the built bootloader into the ESP directory, where the firmware can find it.
    cp ../target/x86_64-unknown-uefi/release/bootloader.efi esp/efi/boot/bootx64.efi

# Run the virtual machine (QEMU).
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
