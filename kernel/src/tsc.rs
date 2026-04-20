//! # Timestamp Counter (TSC)

use {log::info, time::FEMTOS_PER_MICRO};

pub static mut TSC_PERIOD: u64 = 0;


pub fn init() {
    info!("Initializing TSC...");

    let start = read();
    pit::sleep(10_000);
    let end = read();

    let period = (FEMTOS_PER_MICRO * 10_000) / (end - start);

    info!("TSC period: ~{period}fs",);

    unsafe {
        TSC_PERIOD = period;
        time::set_monotonic_clock_period(period);
    }

    let tsc_interval = time::now().elapsed();
    info!("Using TSC as monotonic clock, interval is {tsc_interval:?}");
}

#[inline(always)]
pub fn read() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}
