//! # Hardware Setup
//!
//! **Everything you should be able to know at compile time, but can't.**
//!
//! This module is just a set of statics that dependent modules can bind to at compile
//! time to get information about the user's hardware setup. When the kernel starts up, it
//! sets these variables to the correct values. Userspace programs never see invalid
//! values.
//!
//! Because modules are unlinked object files, all accesses to these values are stored as
//! relocations which are then linked at load time. This is effectively the equivalent of
//! defining constants with the user's _actual_ hardware information.

#![no_std]

/// The frequency, in kilohertz, of the time-stamp counter (TSC).
#[cfg(target_arch = "x86_64")]
#[used]
pub static mut TSC_FREQUENCY_KHZ: u64 = 0;

/// The period, in femtoseconds, of the time-stamp counter (TSC).
#[cfg(target_arch = "x86_64")]
#[used]
pub static mut TSC_PERIOD_FS: u64 = 0;
