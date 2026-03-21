//! # High Precision Event Timer (HPET)

use {
    acpi::sdt::hpet::HpetTable,
    core::ptr::{read_volatile, write_volatile},
    log::{debug, info},
};


/// Offset of the *General Capabilities and ID* register.
const GENERAL_CAPABILITIES_OFFSET: u64 = 0x000;
/// Offset of the *General Gonfiguration* register.
const GENERAL_CONFIG_OFFSET: u64 = 0x010;
/// Offset of the *Main Counter Value* register.
const MAIN_COUNTER_OFFSET: u64 = 0x0F0;

const CONFIG_ENABLE_BIT: u64 = 0b_0001;

static mut HPET_ADDR: u64 = 0;

pub fn init(hpet: &HpetTable) {
    unsafe {
        HPET_ADDR = hpet.base_address.address;

        info!("Initializing HPET @ {:#x}...", HPET_ADDR);

        {
            let capabilities = read_reg(GENERAL_CAPABILITIES_OFFSET);
            let period = (capabilities >> 32) / 1_000_000;
            let id = (capabilities >> 16) as u16;
            let revision = capabilities as u8;
            let timer_count = ((capabilities >> 8) as u8 & 0x1F) + 1;

            debug!(
                "\n\tCapabilities: {capabilities:#x}\n\
                \t    id: {id:#x} (rev: {revision})\n\
                \t    clock period: {period}ns\n\
                \t    timer count: {timer_count}",
            );
        }

        // Disable the HPET.
        {
            let mut config = read_reg(GENERAL_CONFIG_OFFSET);
            config &= !CONFIG_ENABLE_BIT;
            write_reg(GENERAL_CONFIG_OFFSET, config);
        }

        // TODO: Configure the HPET here.

        // Enable the HPET.
        {
            let mut config: u64 = read_reg(GENERAL_CONFIG_OFFSET);
            config |= CONFIG_ENABLE_BIT;
            write_reg(GENERAL_CONFIG_OFFSET, config);
        }
    }
}

pub fn available() -> bool {
    unsafe { HPET_ADDR != 0 }
}

pub struct HpetClock;

impl time::ClockMonotonic for HpetClock {
    #[inline(always)]
    fn now() -> u64 {
        unsafe { read_reg(MAIN_COUNTER_OFFSET) }
    }

    #[inline(always)]
    fn period() -> u64 {
        unsafe { read_reg(GENERAL_CAPABILITIES_OFFSET) >> 32 }
    }
}

#[inline(always)]
unsafe fn read_reg(offset: u64) -> u64 {
    unsafe { read_volatile((HPET_ADDR + offset) as *const u64) }
}

#[inline(always)]
unsafe fn write_reg(offset: u64, value: u64) {
    unsafe {
        write_volatile((HPET_ADDR + offset) as *mut u64, value);
    }
}
