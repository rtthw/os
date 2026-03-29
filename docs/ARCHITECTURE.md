


# Architecture

The project consists of 3 major components:

- Bare metal operating system kernel (in [/kernel](../kernel)).
- UEFI Bootloader (in [/bootloader](../bootloader)).
- Linux distribution (in [/linux](../linux), with an example application in [/example](../example)).

> [!NOTE]
> The Linux distribution is likely to be deprecated once the kernel is in a usable state. Not that it ever worked properly anyway.

The [/crates](../crates) directory contains various crates (e.g. [`boot-info`](../crates/boot-info)) used by one or more of the major components.

The [/drivers](../drivers) directory contains device drivers used by the kernel.

## Current State

✔️ Up To Date, ❌ Stale

- [/bootloader](../bootloader) ✔️
- [/crates](../crates)
  - [/abi](../crates/abi) ❌
  - [/abi-tests](../crates/abi-tests) ✔️
  - [/bit-utils](../crates/bit-utils) ✔️
  - [/boot-info](../crates/boot-info) ✔️
  - [/driver](../crates/driver) ❌
  - [/elf](../crates/elf) ✔️
  - [/emulator](../crates/emulator) ✔️
  - [/framebuffer](../crates/framebuffer) ✔️
  - [/loader](../crates/loader) ✔️
  - [/log](../crates/log) ✔️
  - [/memory-types](../crates/memory-types) ✔️
  - [/pod](../crates/pod) ✔️
  - [/spin-mutex](../crates/spin-mutex) ✔️
  - [/time](../crates/time) ✔️
- [/drivers](../drivers)
  - [/pci](../crates/pci) ✔️
  - [/virtio](../crates/virtio) ✔️
- [/example](../example) ❌
- [/kernel](../kernel) ✔️
- [/linux](../linux) ❌
