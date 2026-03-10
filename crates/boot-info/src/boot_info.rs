//! # Boot Information
//!
//! Shared by the boot loader and kernel.

#![no_std]

use core::ops::{Deref, DerefMut};



/// Information passed from the boot loader to the kernel when the OS boots up.
#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub rsdp_address: Option<u64>,
    pub kernel_start: usize,
    pub kernel_end: usize,
    pub memory_map: MemoryMap,
}

#[derive(Debug)]
#[repr(C)]
pub struct MemoryMap {
    ptr: *mut MemoryRegion,
    len: usize,
}

impl Deref for MemoryMap {
    type Target = [MemoryRegion];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl DerefMut for MemoryMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl From<&'static mut [MemoryRegion]> for MemoryMap {
    fn from(regions: &'static mut [MemoryRegion]) -> Self {
        Self {
            ptr: regions.as_mut_ptr(),
            len: regions.len(),
        }
    }
}

impl From<MemoryMap> for &'static mut [MemoryRegion] {
    fn from(map: MemoryMap) -> &'static mut [MemoryRegion] {
        unsafe { core::slice::from_raw_parts_mut(map.ptr, map.len) }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct MemoryRegion {
    pub base: usize,
    pub size: usize,
    pub kind: MemoryRegionKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
#[repr(C)]
pub enum MemoryRegionKind {
    Free,
    Bootloader,
    Uefi(u32),
}
