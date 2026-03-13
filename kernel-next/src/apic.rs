//! # Advanced Programmable Interrupt Controller (APIC)

use {
    acpi::platform::interrupt::Apic,
    core::sync::atomic::{AtomicUsize, Ordering},
    log::info,
    spin_mutex::Mutex,
    x2apic::lapic::{self, LocalApic},
    x86_64::structures::idt::InterruptStackFrame,
};



pub const TIMER_INDEX: u8 = 32;
pub const ERROR_INDEX: u8 = 32 + 19;
pub const SPURIOUS_INDEX: u8 = 32 + 31;

const TIMER_INTERVAL: u32 = 10_000_000;

static mut LOCAL_APIC: Mutex<Option<LocalApic>> = Mutex::new(None);
static TICKS: AtomicUsize = AtomicUsize::new(0);


pub fn init(info: Apic) {
    info!("Initializing APIC...");

    #[allow(static_mut_refs)]
    unsafe {
        *LOCAL_APIC.lock() = Some(
            lapic::LocalApicBuilder::new()
                .error_vector(ERROR_INDEX as usize)
                .spurious_vector(SPURIOUS_INDEX as usize)
                .timer_vector(TIMER_INDEX as usize)
                .timer_divide(lapic::TimerDivide::Div16)
                .timer_initial(TIMER_INTERVAL)
                .set_xapic_base(info.local_apic_address)
                .build()
                .expect("failed to build lapic"),
        );

        LOCAL_APIC
            .lock()
            .as_mut()
            .expect("APIC initialized")
            .enable();
    }
}

pub extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // log::debug!("APIC_TICK");
    TICKS.fetch_add(1, Ordering::Relaxed);
    end_of_interrupt();
}

fn end_of_interrupt() {
    #[allow(static_mut_refs)]
    unsafe {
        LOCAL_APIC
            .lock()
            .as_mut()
            .expect("APIC initialized")
            .end_of_interrupt()
    };
}
