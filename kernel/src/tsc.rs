//! # Timestamp Counter (TSC)

use {
    crate::pit,
    log::info,
    time::{ClockMonotonic, FEMTOS_PER_MICRO},
};

static mut TSC_PERIOD: u64 = 0;


pub fn init() {
    info!("Initializing TSC...");

    let start = read();
    pit::sleep(10_000);
    let end = read();

    let period = (FEMTOS_PER_MICRO * 10_000) / (end - start);

    info!("TSC period: ~{period}fs",);

    unsafe {
        TSC_PERIOD = period;
    }
}

#[inline(always)]
pub fn read() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}

pub struct TscClock;

impl ClockMonotonic for TscClock {
    #[inline(always)]
    fn now() -> u64 {
        read()
    }

    fn period() -> u64 {
        unsafe { TSC_PERIOD }
    }
}
