//! # Boot Information
//!
//! Shared by the boot loader and kernel.

#![no_std]



/// Information passed from the boot loader to the kernel when the OS boots up.
#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub rsdp_address: Option<u64>,
    pub kernel_start: usize,
    pub kernel_end: usize,
}
