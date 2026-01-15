#!/bin/sh

# Make a static home directory for use with QEMU.
#
# This exists solely because QEMU limits the maximum size of a virtual disk
# image (the `build/rootfs/` directory) to about 500 MB, which is too small.

cd build

# Create the disk image file.
dd if=/dev/zero of=home.img bs=1M count=2048

# Convert it into a filesystem (requires root privileges).
sudo mkfs.ext4 -d homefs home.img
