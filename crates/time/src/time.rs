//! # Time

#![no_std]

use core::sync::atomic::{AtomicBool, Ordering};

pub use core::time::*;

/// The amount of femtoseconds in a nanosecond.
pub const FEMTOS_PER_NANO: u128 = 1_000_000;

static MONOTONIC_CLOCK_SET: AtomicBool = AtomicBool::new(false);

static mut MONOTONIC_NOW: fn() -> u64 = dummy_monotonic_now;
static mut MONOTONIC_PERIOD: u64 = 1;

pub trait ClockMonotonic {
    /// The current clock value.
    fn now() -> u64;
    /// The period of this clock in femtoseconds.
    fn period() -> u64;
}

/// Set the global monotonic clock.
pub unsafe fn set_monotonic_clock<C: ClockMonotonic>() {
    unsafe {
        MONOTONIC_NOW = C::now;
        MONOTONIC_PERIOD = C::period();
    }
    MONOTONIC_CLOCK_SET.store(true, Ordering::Relaxed);
}

/// Returns `true` if the global monotonic clock has been set.
pub fn monotonic_clock_ready() -> bool {
    MONOTONIC_CLOCK_SET.load(Ordering::Relaxed)
}

fn dummy_monotonic_now() -> u64 {
    panic!("called `MONOTONIC_NOW` before a monotonic clock was set")
}

/// An alias for [`Instant::now`].
#[inline(always)]
pub fn now() -> Instant {
    Instant::now()
}

#[repr(transparent)]
pub struct Instant {
    value: u64,
}

impl Instant {
    /// Get the current monotonic clock value.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// use time::*;
    /// let earlier = Instant::now();
    /// let later = Instant::now();
    /// assert!(later.duration_since(earlier) > Duration::ZERO);
    /// ```
    ///
    /// ## Panics
    ///
    /// This function will panic if the global monotonic clock has not been set
    /// by the kernel.
    pub fn now() -> Self {
        Self {
            value: unsafe { MONOTONIC_NOW() },
        }
    }

    pub fn duration_since(&self, earlier: Self) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    pub fn checked_duration_since(&self, earlier: Self) -> Option<Duration> {
        let instant = Instant {
            value: self.value.checked_sub(earlier.value)?,
        };
        let femtos = instant.value as u128 * unsafe { MONOTONIC_PERIOD as u128 };

        Some(Duration::from_nanos((femtos / FEMTOS_PER_NANO) as u64))
    }
}
