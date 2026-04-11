//! # Deferring Mutex
//!
//! A [`Mutex`](spin_mutex::Mutex) that calls [`defer`] if it can't acquire the
//! lock.

#![no_std]

use spin_mutex::{Guard, Mutex};



/// A [`Mutex`](spin_mutex::Mutex) that calls [`defer`] if it can't acquire the
/// lock.
pub struct DeferMutex<T: ?Sized> {
    inner: Mutex<T>,
}

impl<T> DeferMutex<T> {
    /// Create a new mutex with the given data.
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self {
            inner: Mutex::new(data),
        }
    }
}

impl<T: ?Sized> DeferMutex<T> {
    /// Try to lock the mutex, [deferring execution](defer) until the lock
    /// becomes available.
    #[inline(always)]
    pub fn lock(&self) -> Guard<'_, T> {
        loop {
            if let Some(guard) = self.inner.try_lock_weak() {
                break guard;
            }
            while self.inner.is_locked() {
                defer();
            }
        }
    }
}

/// Defer execution to the scheduler.
// FIXME: Should this call some standard library function? What if the defer
// interrupt number changes?
#[inline(always)]
pub fn defer() {
    unsafe {
        core::arch::asm!("int 0x40");
    }
}
