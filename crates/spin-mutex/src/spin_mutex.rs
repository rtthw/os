//! # Spin-based Mutex
//!
//! Adapted from [the `spin` crate](https://crates.io/crates/spin).

#![no_std]

use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};



pub struct Mutex<T: ?Sized> {
    /// When this is `true`, the mutex is locked. When this is `false`, the
    /// mutex is unlocked.
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct Guard<'lock, T: ?Sized + 'lock> {
    lock: &'lock AtomicBool,
    data: *mut T,
}

const LOCKED: bool = true;
const UNLOCKED: bool = false;

impl<T> Mutex<T> {
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(UNLOCKED),
            data: UnsafeCell::new(data),
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> T {
        // SAFETY: No need to lock because there are no other references to `self`.
        let Self { data, .. } = self;

        data.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }

    #[inline(always)]
    pub unsafe fn force_unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    #[inline(always)]
    pub fn lock(&self) -> Guard<'_, T> {
        // NOTE: Can fail to lock even if the spinlock is not locked. May be more
        //       efficient than `try_lock` when called in a loop.
        loop {
            if let Some(guard) = self.try_lock_weak() {
                break guard;
            }
            while self.is_locked() {
                core::hint::spin_loop();
            }
        }
    }

    #[inline(always)]
    pub fn try_lock(&self) -> Option<Guard<'_, T>> {
        // NOTE: We use a strong `cmpxchg` here because on some platforms (e.g. ARM) a
        //       weak `cmpxchg` can fail for arbitrary reasons (like a timer interrupt),
        //       even if the lock is currently free. Even though we don't (yet) support
        //       ARM, it's good practice. See:
        //       https://github.com/Amanieu/parking_lot/pull/207#issuecomment-575869107
        if self
            .lock
            .compare_exchange(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(Guard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn try_lock_weak(&self) -> Option<Guard<'_, T>> {
        if self
            .lock
            .compare_exchange_weak(UNLOCKED, LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(Guard {
                lock: &self.lock,
                data: unsafe { &mut *self.data.get() },
            })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        // SAFETY: No need to lock because there are no other references to `self`.
        unsafe { &mut *self.data.get() }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (&*guard).fmt(f))
                .and_then(|()| write!(f, " }}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<'lock, T: ?Sized> Deref for Guard<'lock, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        // SAFETY: Already locked.
        unsafe { &*self.data }
    }
}

impl<'lock, T: ?Sized> DerefMut for Guard<'lock, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: Already locked.
        unsafe { &mut *self.data }
    }
}

impl<'lock, T: ?Sized> Drop for Guard<'lock, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}

unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

unsafe impl<T: ?Sized + Sync> Sync for Guard<'_, T> {}
unsafe impl<T: ?Sized + Send> Send for Guard<'_, T> {}



#[cfg(test)]
mod tests {
    use super::*;

    extern crate std;

    use std::{
        prelude::v1::*,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
            mpsc::channel,
        },
        thread,
    };

    #[test]
    fn smoke() {
        let m = Mutex::<_>::new(());
        drop(m.lock());
        drop(m.lock());
    }

    #[test]
    fn stress() {
        static M: Mutex<()> = Mutex::<_>::new(());
        static mut COUNT: u32 = 0;
        const J: u32 = 1000;
        const K: u32 = 3;

        fn inc() {
            for _ in 0..J {
                unsafe {
                    let _g = M.lock();
                    COUNT += 1;
                }
            }
        }

        let (tx, rx) = channel();
        let mut ts = Vec::new();
        for _ in 0..K {
            let tx2 = tx.clone();
            ts.push(thread::spawn(move || {
                inc();
                tx2.send(()).unwrap();
            }));
            let tx2 = tx.clone();
            ts.push(thread::spawn(move || {
                inc();
                tx2.send(()).unwrap();
            }));
        }

        drop(tx);
        for _ in 0..2 * K {
            rx.recv().unwrap();
        }
        assert_eq!(unsafe { COUNT }, J * K * 2);

        for t in ts {
            t.join().unwrap();
        }
    }

    #[test]
    fn try_lock() {
        let mutex = Mutex::<_>::new(43);

        // First lock succeeds
        let a = mutex.try_lock();
        assert_eq!(a.as_ref().map(|r| **r), Some(43));

        // Additional lock fails
        let b = mutex.try_lock();
        assert!(b.is_none());

        // After dropping lock, it succeeds again
        ::core::mem::drop(a);
        let c = mutex.try_lock();
        assert_eq!(c.as_ref().map(|r| **r), Some(43));
    }

    #[test]
    fn into_inner() {
        #[derive(Debug, PartialEq)]
        struct NonCopy(u8);

        let m = Mutex::<_>::new(NonCopy(43));
        assert_eq!(m.into_inner(), NonCopy(43));
    }

    #[test]
    fn into_inner_drop() {
        struct Foo(Arc<AtomicUsize>);

        impl Drop for Foo {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let drop_count = Arc::new(AtomicUsize::new(0));
        let mutex = Mutex::<_>::new(Foo(drop_count.clone()));

        assert_eq!(drop_count.load(Ordering::SeqCst), 0);
        {
            let _inner = mutex.into_inner();
            assert_eq!(drop_count.load(Ordering::SeqCst), 0);
        }
        assert_eq!(drop_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn nested() {
        let inner: Arc<Mutex<i32>> = Arc::new(Mutex::<_>::new(1));
        let outer: Arc<Mutex<Arc<Mutex<i32>>>> = Arc::new(Mutex::<_>::new(inner));

        let (send, recv) = channel();
        let thread = thread::spawn(move || {
            let outer_lock: Guard<'_, Arc<Mutex<i32>>> = outer.lock();
            let inner_lock: Guard<'_, i32> = outer_lock.lock();
            assert_eq!(*inner_lock, 1);
            send.send(()).unwrap();
        });

        recv.recv().unwrap();
        thread.join().unwrap();
    }

    #[test]
    fn access_during_unwind() {
        let value = Arc::new(Mutex::<_>::new(2));
        let value_clone = value.clone();

        _ = thread::spawn(move || {
            struct Unwinder {
                i: Arc<Mutex<i32>>,
            }

            impl Drop for Unwinder {
                fn drop(&mut self) {
                    *self.i.lock() += 1;
                }
            }

            let _ = Unwinder { i: value_clone };
            self::panic!();
        })
        .join();

        let lock = value.lock();
        assert_eq!(*lock, 3);
    }

    #[test]
    fn unsized_data() {
        let mutex: &Mutex<[u8]> = &Mutex::<_>::new([2, 3, 5]);
        {
            let b = &mut *mutex.lock();
            b[0] = 7;
            b[2] = 11;
        }
        assert_eq!(&*mutex.lock(), &[7, 3, 11]);
    }

    #[test]
    fn force_unlock() {
        let lock = Mutex::<_>::new(());
        core::mem::forget(lock.lock());
        unsafe {
            lock.force_unlock();
        }
        assert!(lock.try_lock().is_some());
    }
}
