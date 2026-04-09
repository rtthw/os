#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(static_mut_refs)]

#[macro_use]
extern crate alloc;

mod acpi;
mod apic;
mod ata;
mod defer_mutex;
mod executor;
mod gdt;
mod hpet;
mod idt;
mod input;
mod loader;
mod memory;
mod pit;
mod rtc;
mod scheduler;
mod serial;
mod tsc;
mod vfat;
mod window_manager;

use {
    alloc::{string::String, vec::Vec},
    boot_info::BootInfo,
    core::{arch::asm, time::Duration},
    log::{debug, info, warn},
    memory_types::PAGE_SIZE,
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

pub extern "sysv64" fn main(boot_info: &'static BootInfo) -> ! {
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

        // info!("STACK @ {stack_addr:#x}, OBJ @ {stack_object_addr:#x}");

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

    init_monotonic_clock();
    assert!(time::monotonic_clock_ready());

    let pm_start = time::now();
    if let Ok(()) = acpi::pm_timer_sleep(1_000) {
        let dur = pm_start.elapsed();
        debug!("`acpi::pm_timer_sleep(1ms)`\t: {dur:?}");
    }
    let pit_start = time::now();
    pit::sleep(1_000);
    let dur = pit_start.elapsed();
    debug!("`pit::sleep(1ms)`\t\t: {dur:?}");

    ata::init();

    // info!("PCI Devices:");
    // for pci_device in pci::enumerate_devices() {
    //     debug!("{pci_device:#?}");
    // }

    unsafe {
        BOOT_INFO = Some(boot_info);
    }

    info!("STARTUP SUCCESSFUL");

    scheduler::with_scheduler(|scheduler| {
        scheduler.run_process(
            "init",
            init_process as *const fn() -> !,
            Some(PAGE_SIZE * 32),
        )
    });
    scheduler::with_scheduler(|scheduler| {
        scheduler.run_process(
            "clock_update_dispatcher",
            dispatch_clock_updates as *const fn() -> !,
            None,
        )
    });
    scheduler::with_scheduler(|scheduler| {
        scheduler.run_process(
            "input_event_dispatcher",
            input::dispatch_input_events as *const fn() -> !,
            Some(PAGE_SIZE * 32),
        )
    });

    window_manager::init();

    scheduler::run()
}

static mut BOOT_INFO: Option<&'static BootInfo> = None;

fn init_process() -> ! {
    let mut executor = executor::Executor::new();
    loop {
        executor.tick();
        scheduler::defer();
    }
}

fn dispatch_clock_updates() -> ! {
    loop {
        let start = time::now();
        window_manager::send_event(window_manager::Event::ClockUpdate);
        while time::now().duration_since(start) < Duration::from_secs(1) {
            scheduler::defer();
        }
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    loop {}
}



const KERNEL_STACK_SIZE: usize = 16 * PAGE_SIZE;
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



fn init_monotonic_clock() {
    const MAX_USABLE_MONOCLOCK_INTERVAL: Duration = Duration::from_micros(1);

    unsafe {
        time::set_monotonic_clock::<tsc::TscClock>();
    }

    let tsc_interval = time::now().elapsed();
    if tsc_interval <= MAX_USABLE_MONOCLOCK_INTERVAL {
        info!("Using TSC as monotonic clock, interval is {tsc_interval:?}");
        return;
    } else {
        warn!("TSC is too slow for accurate time measurements: {tsc_interval:?}");
    }

    if hpet::available() {
        unsafe {
            time::set_monotonic_clock::<hpet::HpetClock>();
        }
        let hpet_interval = time::now().elapsed();
        if hpet_interval <= MAX_USABLE_MONOCLOCK_INTERVAL {
            info!("Using HPET as monotonic clock, interval is {hpet_interval:?}");
            return;
        }

        warn!(
            "No adequate monotonic clock available, using fastest one...\n\
            \tTSC: {tsc_interval:?}\n\
            \tHPET: {hpet_interval:?}"
        );

        if hpet_interval >= tsc_interval {
            unsafe {
                time::set_monotonic_clock::<tsc::TscClock>();
            }
        }
    }
}



pub trait FileSystem {
    fn list(&self, dir_path: &str) -> Result<Vec<String>, &'static str>;
    fn read(&mut self, path: &str) -> Result<Vec<u8>, &'static str>;
}

pub struct DirectoryEntry {
    pub index: usize,
    pub name: String,
    pub size: usize,
}
