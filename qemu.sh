#!/bin/sh


# Build the init program.
cargo build --release --target x86_64-unknown-linux-musl --bin init || exit

cd build

# Create the initial RAM disk.
mkdir -p initrd
cd initrd

    # Create the core directories.
    mkdir -p bin dev etc lib proc run sbin sys

    # Put the init program where the kernel can find it.
    rm sbin/init
    cp ../../target/x86_64-unknown-linux-musl/release/init sbin/
    chmod +x sbin/init
    ln -s sbin/init init 2>/dev/null

    # Populate the image.
    find | cpio -o -H newc > ../initrd.cpio

cd ..

# Run QEMU.
qemu-system-x86_64 -kernel bzImage -initrd initrd.cpio -append 'console=ttyS0'
