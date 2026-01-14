#!/bin/sh


# Build the init program.
cargo build --release --target x86_64-unknown-linux-musl --bin init || exit
cargo build --release --target x86_64-unknown-linux-gnu --bin shell || exit

# Build the testing program.
cd testing
    cargo build --release --target x86_64-unknown-linux-gnu || exit
cd ..

cd build

# Create the initial RAM disk.
mkdir -p initrd
cd initrd

    # Create the core directories.
    mkdir -p dev proc sbin sys
    mkdir -p ../rootfs/sbin ../rootfs/home/bin

    # Put the init program where the kernel can find it.
    cp ../../target/x86_64-unknown-linux-musl/release/init sbin/
    chmod +x sbin/init
    cp sbin/init ../rootfs/sbin/
    ln -s sbin/init init 2>/dev/null

    # Put the shell program where the init program can find it.
    cp ../../target/x86_64-unknown-linux-gnu/release/shell ../rootfs/sbin/
    chmod +x ../rootfs/sbin/shell

    # Put the testing program where the shell can find it.
    cp ../../target/x86_64-unknown-linux-gnu/release/libtesting.so ../rootfs/home/bin/

    # Populate the image.
    find | cpio -o -H newc | gzip -1 -n > ../initrd.cpio

cd ..

# Run QEMU.
qemu-system-x86_64 \
    -m 2G \
    -smp 4 \
    -vga none \
    -device virtio-gpu-gl \
    -display gtk,gl=on,show-tabs=on,show-cursor=on \
    -usbdevice tablet \
    -drive file=fat:rw:rootfs,format=raw \
    -kernel bzImage \
    -initrd initrd.cpio \
    -append 'console=ttyS0'
