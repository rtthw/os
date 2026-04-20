//! # High Precision Event Timer (HPET)

use {
    acpi::sdt::hpet::HpetTable,
    bit_utils::bit_field,
    core::ptr::{read_volatile, write_volatile},
    log::{debug, info},
};


/// Offset of the *General Capabilities and ID* register.
const GENERAL_CAPABILITIES_OFFSET: u64 = 0x000;
/// Offset of the *General Gonfiguration* register.
const GENERAL_CONFIG_OFFSET: u64 = 0x010;
// /// Offset of the *Main Counter Value* register.
// const MAIN_COUNTER_OFFSET: u64 = 0x0F0;

const CONFIG_ENABLE_BIT: u64 = 0b_0001;

static mut HPET_ADDR: u64 = 0;


pub fn init(hpet: &HpetTable) {
    unsafe {
        HPET_ADDR = hpet.base_address.address;

        info!("Initializing HPET @ {:#x}...", HPET_ADDR);

        {
            let capabilities = GeneralCapabilities::read();
            let caps_num = capabilities.0;
            let period = capabilities.period() / 1_000_000;
            let vendor_id = capabilities.vendor_id();
            let revision = capabilities.revision();
            let timer_count = capabilities.timer_count() + 1;

            debug!(
                "\n\tCapabilities: {caps_num:#x}\n\
                \t    vendor: {vendor_id:#x} (rev: {revision})\n\
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

bit_field! {
    /// See: https://wiki.osdev.org/HPET#General_Capabilities_and_ID_Register
    pub struct GeneralCapabilities: u64 {
        /// Indicates which revision of the function is implemented; must not be zero.
        pub revision: u8 = 0..8,
        /// The amount of timers - 1.
        pub timer_count: u8 = 8..13,
        /// Whether the HPET main counter is capable of operating in 64-bit mode.
        pub counter_is_64_bit: bool = 13,
        /// Whether the HPET is capable of using "legacy replacement" mapping.
        pub can_use_legacy_replacement: bool = 15,
        /// This field should be interpreted similarly to PCI's vendor ID.
        pub vendor_id: u16 = 16..32,
        /// Main counter tick period in femtoseconds (10^-15 seconds). Must not be zero,
        /// must be less or equal to 0x05F5E100, or 100 nanoseconds.
        pub period: u32 = 32..64,
    }
}

impl GeneralCapabilities {
    pub unsafe fn read() -> Self {
        Self(unsafe { read_reg(GENERAL_CAPABILITIES_OFFSET) })
    }
}
