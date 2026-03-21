//! # Advanced Programmable Interrupt Controller (APIC)

use {
    crate::{pit, rtc},
    acpi::platform::interrupt::Apic,
    core::sync::atomic::{AtomicU64, Ordering},
    log::{debug, info, warn},
    spin_mutex::Mutex,
    x2apic::lapic::{self, LocalApic},
    x86_64::structures::idt::InterruptStackFrame,
};



pub const TIMER_INTERVAL_MICROS: u16 = 10_000; // 10 milliseconds
pub const TIMER_INTERVAL_MILLIS: u16 = TIMER_INTERVAL_MICROS / 1_000;

pub const TIMER_INDEX: u8 = 32;
pub const ERROR_INDEX: u8 = 32 + 19;
pub const SPURIOUS_INDEX: u8 = 32 + 31;

static mut LOCAL_APIC: Mutex<Option<LocalApic>> = Mutex::new(None);
static TICKS: AtomicU64 = AtomicU64::new(0);


pub fn init(info: Apic) {
    info!("Initializing APIC...");

    unsafe {
        *LOCAL_APIC.lock() = Some(
            lapic::LocalApicBuilder::new()
                .error_vector(ERROR_INDEX as usize)
                .spurious_vector(SPURIOUS_INDEX as usize)
                .timer_vector(TIMER_INDEX as usize)
                .set_xapic_base(info.local_apic_address)
                .build()
                .expect("failed to build lapic"),
        );

        enable(LOCAL_APIC.lock().as_mut().expect("LAPIC exists"));
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

unsafe fn enable(apic: &mut LocalApic) {
    info!("Enabling APIC...");

    // FIXME: I get a GP fault on the first `set_timer_initial` call. Not sure why.
    unsafe {
        // let apic_period = calculate_timer_period(apic, TIMER_INTERVAL_MICROS);
        // debug!("APIC_{}, timer period: {}", apic.id(), apic_period);
        // apic.set_timer_initial(apic_period);
        // apic.set_timer_divide(lapic::TimerDivide::Div16);
        apic.enable();
    }
}

fn calculate_timer_period(apic: &mut LocalApic, microseconds: u16) -> u32 {
    let initial_count = u32::MAX;
    let final_count = unsafe {
        apic.set_timer_initial(initial_count);
        apic.set_timer_divide(lapic::TimerDivide::Div16);
        apic.enable_timer();

        pit::sleep(microseconds);

        apic.disable_timer();

        apic.timer_current()
    };

    initial_count - final_count
}

pub fn current_tick() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

pub fn sleep(milliseconds: u32) {
    let ticks = milliseconds / TIMER_INTERVAL_MILLIS as u32;
    let start_tick = current_tick();
    while current_tick() - start_tick < ticks as u64 {
        core::hint::spin_loop();
    }
}

#[allow(unused)]
pub fn timer_accuracy_tests() {
    info!("Testing timer accuracy...");

    let start = rtc::Time::now();
    let raw_start = unsafe { rtc::Time::now_unsynced() };
    info!("START @ {start} ({raw_start})\n\tSleeping for 60 seconds...");
    sleep(1_000 * 60);
    let raw_end = unsafe { rtc::Time::now_unsynced() };
    let end = rtc::Time::now();
    info!("END @ {end} ({raw_end})");

    for interval in 1..=10 {
        let (start_minute, start_second) = unsafe { rtc::raw_minute_and_second() };
        let start_time = start_second as u32 + (start_minute as u32 * 60);
        sleep(interval * 1_000);
        let (current_minute, current_second) = unsafe { rtc::raw_minute_and_second() };
        let current_time = current_second as u32 + (current_minute as u32 * 60);
        let time_taken = current_time - start_time;
        if time_taken != interval {
            if time_taken.saturating_sub(1) != interval {
                warn!("Timer failed at {interval} second intervals");
            } else {
                info!("Timer slightly off at {interval} second intervals");
            }
        } else {
            info!("Timer accurate at {interval} second intervals");
        }
    }
}
