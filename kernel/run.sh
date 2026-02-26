#!/bin/bash

# FIXME: This only works on Linux-based systems.

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

cargo build --release --target x86_64-unknown-uefi
mkdir -p esp/efi/boot
cp ../target/x86_64-unknown-uefi/release/kernel.efi esp/efi/boot/bootx64.efi

qemu-system-x86_64 \
    -m 1G \
    -rtc base=utc \
    -display sdl \
    -drive if=pflash,format=raw,readonly=on,file=firmware/uefi/OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=firmware/uefi/OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -device virtio-keyboard \
    -device virtio-mouse \
    -device virtio-net-pci,netdev=network0 -netdev user,id=network0 \
    -vga virtio \
    -serial stdio
