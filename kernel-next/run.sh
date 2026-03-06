#!/bin/bash

set -ex

cd ../bootloader
    cargo build --release
cd ../kernel-next
    cargo build --release
    mkdir -p esp/efi/boot
    cp ../target/x86_64-unknown-uefi/release/bootloader.efi esp/efi/boot/bootx64.efi
    cp ../target/x86_64-kernel/release/kernel-next esp/kernel

qemu-system-x86_64 \
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
