//! # Timestamp Counter (TSC)

use {log::info, time::FEMTOS_PER_MILLI};



pub fn init() {
    info!("Initializing TSC...");

    let cpuid = raw_cpuid::CpuId::new();
    if !cpuid
        .get_feature_info()
        .map_or(false, |info| info.has_tsc())
    {
        panic!("x86 CPU should have a TSC!")
    }

    let frequency_khz = {
        if let Some(info) = cpuid.get_tsc_info() {
            if info.nominal_frequency() != 0 {
                info.tsc_frequency()
            } else if info.numerator() != 0 && info.denominator() != 0 {
                cpuid
                    .get_processor_frequency_info()
                    .map(|info| info.processor_base_frequency() as u64 * 1000)
                    .map(|cpu_base_freq_hz| {
                        let crystal_hz =
                            cpu_base_freq_hz * info.denominator() as u64 / info.numerator() as u64;
                        crystal_hz * info.numerator() as u64 / info.denominator() as u64
                    })
            } else {
                None
            }
        } else if let Some(info) = cpuid.get_hypervisor_info() {
            log::trace!("Using TSC frequency from hypervisor");
            info.tsc_frequency().map(|tsc_khz| tsc_khz as u64)
        } else {
            None
        }
    }
    .unwrap_or({
        log::trace!("Guessing TSC frequency...");
        let start = read();
        pit::sleep(10_000);
        let end = read();
        (end - start) / 10
    });

    let period_fs = FEMTOS_PER_MILLI / frequency_khz;

    info!("TSC frequency: {frequency_khz} KHz");
    info!("TSC period: {period_fs} fs");

    unsafe {
        hardware::TSC_FREQUENCY_KHZ = frequency_khz;
        hardware::TSC_PERIOD_FS = period_fs;
    }

    let tsc_interval = time::now().elapsed();
    info!("Using TSC as monotonic clock, interval is {tsc_interval:?}");
}

#[inline(always)]
pub fn read() -> u64 {
    unsafe { core::arch::x86_64::_rdtsc() }
}
