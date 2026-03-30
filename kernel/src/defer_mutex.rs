//! # Deferring Mutex
//!
//! A [`Mutex`](spin_mutex::Mutex) that calls [`scheduler::defer`] if it can't
//! acquire the lock.

use {
    crate::scheduler,
    spin_mutex::{Guard, Mutex},
};



/// A [`Mutex`](spin_mutex::Mutex) that calls [`scheduler::defer`] if it can't
/// acquire the lock.
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
    /// Try to lock the mutex, [deferring execution](scheduler::defer) until the
    /// lock becomes available.
    #[inline(always)]
    pub fn lock(&self) -> Guard<'_, T> {
        loop {
            if let Some(guard) = self.inner.try_lock_weak() {
                break guard;
            }
            while self.inner.is_locked() {
                scheduler::defer();
            }
        }
    }
}
