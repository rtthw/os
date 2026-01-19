#!/bin/sh


# Build the ABI.
cd abi
    cargo build --release --target x86_64-unknown-linux-gnu || exit
cd ..

# Build the init program.
cargo build --release --target x86_64-unknown-linux-musl --bin init || exit
cargo build --release --target x86_64-unknown-linux-gnu --bin shell || exit

cd build

# Create the initial RAM disk.
mkdir -p initrd
cd initrd

    # Create the core directories.
    mkdir -p dev proc sbin sys
    mkdir -p ../rootfs/sbin ../rootfs/usr/bin

    # Put the init program where the kernel can find it.
    cp ../../target/x86_64-unknown-linux-musl/release/init sbin/
    chmod +x sbin/init
    cp sbin/init ../rootfs/sbin/
    ln -s sbin/init init 2>/dev/null

    # Put the shell program where the init program can find it.
    cp ../../target/x86_64-unknown-linux-gnu/release/shell ../rootfs/sbin/
    chmod +x ../rootfs/sbin/shell

    # Put the ABI where the compiler can find it.
    cp ../../target/x86_64-unknown-linux-gnu/release/libabi.rlib ../rootfs/lib/
    cp ../../abi-tests/src/abi_tests.rs ../rootfs/lib/

    # Put the example program where the compiler can find it.
    cp ../../example/src/example.rs ../rootfs/lib/

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
    -drive file=home.img,format=raw \
    -kernel bzImage \
    -initrd initrd.cpio \
    -append 'console=ttyS0'
