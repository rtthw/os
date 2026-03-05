#!/bin/bash


echo "Building artifacts..."

# Build the ABI.
cd ../crates/abi
    cargo build --release --target x86_64-unknown-linux-gnu || exit
cd ../../linux

# Build the init program.
cargo build --release --target x86_64-unknown-linux-musl --bin init || exit
cargo build --release --target x86_64-unknown-linux-gnu --bin shell || exit
# cargo build --release --target x86_64-unknown-linux-gnu --bin driver || exit

mkdir -p build
cd build

# TODO: Automatically install the Linux kernel here.
if [[ ! -e "bzImage" ]]; then
    echo "File 'build/bzImage' does not exist. Install the Linux kernel, then place it into the build directory to proceed."
    exit 1
fi

if [[ ! -e "home.img" ]]; then
    echo "File 'build/home.img' does not exist. Did you run the 'make_home_image.sh' script?"
    exit 1
fi

echo "Creating the initial RAM disk..."

# Create the initial RAM disk.
mkdir -p initrd
cd initrd
    # Create the core directories.
    mkdir -p dev proc sbin sys
    mkdir -p ../rootfs/sbin ../rootfs/usr/bin ../rootfs/lib ../rootfs/home

    # Put the init program where the kernel can find it.
    cp ../../../target/x86_64-unknown-linux-musl/release/init sbin/
    chmod +x sbin/init
    cp sbin/init ../rootfs/sbin/
    ln -s sbin/init init 2>/dev/null

    # Put the shell program where the init program can find it.
    cp ../../../target/x86_64-unknown-linux-gnu/release/shell ../rootfs/sbin/
    # cp ../../../target/x86_64-unknown-linux-gnu/release/driver ../rootfs/sbin/
    chmod +x ../rootfs/sbin/shell
    # chmod +x ../rootfs/sbin/driver

    # Put the ABI where the compiler can find it.
    cp ../../../target/x86_64-unknown-linux-gnu/release/libabi.rlib ../rootfs/lib/
    cp ../../../crates/abi-tests/src/abi_tests.rs ../rootfs/lib/

    # Put the example program where the compiler can find it.
    cp ../../../example/src/example.rs ../rootfs/lib/

    # Populate the image.
    find | cpio -o -H newc 2>/dev/null | gzip -1 -n > ../initrd.cpio

cd ..

echo "Running QEMU..."

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
