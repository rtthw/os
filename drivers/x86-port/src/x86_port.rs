//! # x86 Port I/O
//!
//! See the [OSDev] and [Wikipedia] articles for more information.
//!
//! [OSDev]: https://wiki.osdev.org/Port_IO
//! [Wikipedia]: https://en.wikipedia.org/wiki/Memory-mapped_I/O_and_port-mapped_I/O

#![no_std]

use core::arch::asm;



/// Read an 8-bit value from the given port.
#[inline]
#[doc(alias = "inb")]
pub unsafe fn read_u8(port: u16) -> u8 {
    let value: u8;
    unsafe {
        asm!(
            "in al, dx",
            out("al") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Read a 16-bit value from the given port.
#[inline]
#[doc(alias = "inw")]
pub unsafe fn read_u16(port: u16) -> u16 {
    let value: u16;
    unsafe {
        asm!(
            "in ax, dx",
            out("ax") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Read a 32-bit value from the given port.
#[inline]
#[doc(alias = "inl")]
pub unsafe fn read_u32(port: u16) -> u32 {
    let value: u32;
    unsafe {
        asm!(
            "in eax, dx",
            out("eax") value,
            in("dx") port,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Write an 8-bit value to the given port.
#[inline]
#[doc(alias = "outb")]
pub unsafe fn write_u8(port: u16, value: u8) {
    unsafe {
        asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Write a 16-bit value to the given port.
#[inline]
#[doc(alias = "outw")]
pub unsafe fn write_u16(port: u16, value: u16) {
    unsafe {
        asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Write a 32-bit value to the given port.
#[inline]
#[doc(alias = "outl")]
pub unsafe fn write_u32(port: u16, value: u32) {
    unsafe {
        asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}
