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

# Build the example program.
cd ..
    cargo rustc \
        --release \
        --manifest-path=crates/time/Cargo.toml \
        --target=kernel/x86_64-app.json \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -- \
        --crate-type=lib \
        --emit=obj="kernel/esp/time.o" \
        -C link-dead-code=yes \
        -Z share-generics=no
    cargo rustc \
        --release \
        --manifest-path=crates/boot-info/Cargo.toml \
        --target=kernel/x86_64-app.json \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -- \
        --crate-type=lib \
        --emit=obj="kernel/esp/boot_info.o" \
        -C link-dead-code=yes \
        -Z share-generics=no
    cargo rustc \
        --release \
        --manifest-path=crates/framebuffer/Cargo.toml \
        --target=kernel/x86_64-app.json \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -- \
        --crate-type=lib \
        --emit=obj="kernel/esp/framebuffer.o" \
        -C link-dead-code=yes \
        -Z share-generics=no
    cargo rustc \
        --release \
        --manifest-path=drivers/pit/Cargo.toml \
        --target=kernel/x86_64-app.json \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -- \
        --crate-type=lib \
        --emit=obj="kernel/esp/pit.o" \
        -C link-dead-code=yes \
        -Z share-generics=no
    cargo rustc \
        --release \
        --manifest-path=example/example-dep/Cargo.toml \
        --target=kernel/x86_64-app.json \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -- \
        --crate-type=lib \
        --emit=obj="kernel/esp/example_dep.o" \
        -C link-dead-code=yes \
        -Z share-generics=no
    cargo rustc \
        --release \
        --manifest-path=example/Cargo.toml \
        --target=kernel/x86_64-app.json \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
        -- \
        --emit=obj="kernel/esp/example.o" \
        -C link-dead-code=yes \
        -Z share-generics=no

    # Create object files for the core language dependencies.
    for path in $(find target/x86_64-app/release/deps/ -name "*.rlib"); do
        filename=$(basename ${path})

        # Strip the `lib` prefix and everything after the last `-`. For example:
        #       `libcore-60553895dc80afc7.rlib` -> `core`
        realname="$(basename ${path} | sed -E 's/^lib(.*)-[^-]*$/\1/')"

        # If the rlib archive is more than just a single object file, it's
        # either core, alloc, or compiler_builtins.
        if [ `ar -t ${path} | wc -l` != "2" ]; then
            echo -e "Extracting '${filename}' to '${realname}.o'"

            # Extract and link all necessary object files together.
            mkdir -p "kernel/tmp/extracted/${filename}"
            ar -xo --output "kernel/tmp/extracted/${filename}/" ${path}
            ld -r \
                --output "kernel/esp/${realname}.o" \
                $(find kernel/tmp/extracted/${filename}/ -name "*.o")
        fi
    done

# Build the kernel.
cd kernel
    cargo rustc \
        --release \
        --manifest-path "$SOURCE_DIR/Cargo.toml" \
        --target "$SOURCE_DIR/x86_64-kernel.json" \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem \
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
    -rtc base=localtime \
    -display gtk,show-tabs=on \
    -drive if=pflash,format=raw,readonly=on,file=firmware/uefi/OVMF_CODE.fd \
    -drive if=pflash,format=raw,readonly=on,file=firmware/uefi/OVMF_VARS.fd \
    -drive format=raw,file=fat:rw:esp \
    -device virtio-keyboard \
    -device virtio-mouse \
    -vga virtio \
    -serial stdio
