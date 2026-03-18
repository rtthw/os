#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

#[macro_use]
extern crate alloc;

mod acpi;
mod apic;
mod gdt;
mod hpet;
mod idt;
mod memory;
mod pit;
mod rtc;
mod serial;
mod tsc;

use {
    boot_info::BootInfo,
    core::arch::asm,
    framebuffer::{Color, Framebuffer},
    log::{debug, info},
    memory_types::MEBIBYTE,
};


unsafe extern "C" {
    static __text_start: u8;
    static __text_end: u8;
    static __rodata_start: u8;
    static __rodata_end: u8;
    static __kernel_end: u8;
}

/// The kernel's entry point.
#[unsafe(no_mangle)]
pub extern "sysv64" fn _start(boot_info: &BootInfo) -> ! {
    unsafe {
        asm!(
            "mov rdi, {}",
            "mov rsp, {}",
            "call {}",
            in(reg) boot_info,
            in(reg) KERNEL_STACK.as_ptr() as u64 + KERNEL_STACK.len() as u64,
            in(reg) main, // See `main` function below.
            options(nomem, nostack),
        );
    }

    unreachable!();
}

pub extern "sysv64" fn main(boot_info: &BootInfo) -> ! {
    let startup_time = rtc::Time::now();

    serial::init();

    info!(
        "KERNEL STARTUP @ {startup_time}\n\
        \ttext: {:#x}..{:#x}\n\
        \trodata: {:#x}..{:#x}\n\
        \tend: {:#x}",
        (&raw const __text_start) as usize,
        (&raw const __text_end) as usize,
        (&raw const __rodata_start) as usize,
        (&raw const __rodata_end) as usize,
        (&raw const __kernel_end) as usize,
    );

    // Make sure `KERNEL_STACK` is actually the current stack.
    {
        let stack_addr = KERNEL_STACK.as_ptr().addr();
        let stack_top_addr = stack_addr + KERNEL_STACK_SIZE;
        let stack_object = boot_info.memory_map.len() + 43;
        let stack_object_addr = ((&stack_object) as *const usize).addr();

        assert!(
            stack_object_addr >= stack_addr && stack_object_addr < stack_top_addr,
            "kernel stack is malformed",
        );
    }

    gdt::init();
    idt::init();
    memory::init(boot_info);
    acpi::init(boot_info);
    tsc::init();

    unsafe {
        time::set_monotonic_clock::<tsc::TscClock>();
        // time::set_monotonic_clock::<hpet::HpetClock>();
    }

    assert!(time::monotonic_clock_ready());

    let time_1 = time::now();
    let dur = time::now().duration_since(time_1);
    debug!("Time between `Instant::now` calls: {dur:?}",);
    let pm_start = time::now();
    if let Ok(()) = acpi::pm_timer_sleep(1_000) {
        let dur = time::now().duration_since(pm_start);
        debug!("`acpi::pm_timer_sleep(1ms)`\t: {dur:?}");
    }
    let pit_start = time::now();
    pit::sleep(1_000);
    let dur = time::now().duration_since(pit_start);
    debug!("`pit::sleep(1ms)`\t\t: {dur:?}");

    x86_64::instructions::interrupts::enable();

    let mut framebuffer = Framebuffer::from_display_info(&boot_info.display_info);
    framebuffer.clear_screen(Color::rgb(0x2B, 0x2B, 0x33));

    for (col, ch) in "KERNEL v0.0.0".char_indices() {
        framebuffer.draw_ascii_char(
            ch,
            Color::rgb(0xaa, 0xaa, 0xad),
            Color::rgb(0x2B, 0x2B, 0x33),
            10,
            10,
            col,
            0,
        );
        framebuffer.draw_ascii_char(
            '-',
            Color::rgb(0xaa, 0xaa, 0xad),
            Color::rgb(0x2B, 0x2B, 0x33),
            10,
            10,
            col,
            1,
        );
    }

    // apic::timer_accuracy_tests();

    info!("STARTUP SUCCESSFUL");

    let clock_start_time = rtc::Time::now();
    let timer_start_tick = apic::current_tick();
    let mut current_time_add = rtc::Time::ZERO;
    loop {
        x86_64::instructions::hlt();
        let ticks_per_second = 100;
        let seconds_since_start = (apic::current_tick() - timer_start_tick) / ticks_per_second;
        let time_since_start = rtc::Time::from_seconds(seconds_since_start);
        if current_time_add != time_since_start {
            current_time_add = time_since_start;
            let current_time = clock_start_time + current_time_add;
            for (col, ch) in format!("Current Time: {current_time}").char_indices() {
                framebuffer.draw_ascii_char(
                    ch,
                    Color::rgb(0xaa, 0xaa, 0xad),
                    Color::rgb(0x2B, 0x2B, 0x33),
                    10,
                    10,
                    col,
                    2,
                );
            }
        }
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    loop {}
}



const KERNEL_STACK_SIZE: usize = 1 * MEBIBYTE;
static KERNEL_STACK: KernelStack = KernelStack::new();

#[repr(align(16))] // System V ABI requires 16 byte stack alignment.
struct KernelStack([u8; KERNEL_STACK_SIZE]);

impl KernelStack {
    const fn new() -> Self {
        Self([0; KERNEL_STACK_SIZE])
    }

    const fn len(&self) -> usize {
        self.0.len()
    }

    const fn as_ptr(&self) -> *const u8 {
        self.0.as_ptr()
    }
}
