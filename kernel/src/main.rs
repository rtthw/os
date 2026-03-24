#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(static_mut_refs)]

#[macro_use]
extern crate alloc;

mod acpi;
mod apic;
mod ata;
mod executor;
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
    core::{arch::asm, time::Duration},
    framebuffer::{Color, Framebuffer},
    log::{debug, info, warn},
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

    let mut page_table = unsafe {
        use x86_64::{
            VirtAddr,
            registers::control::Cr3,
            structures::paging::{OffsetPageTable, PageTable},
        };

        let (l4_frame, _) = Cr3::read();
        let l4_ptr = l4_frame.start_address().as_u64() as *mut PageTable;

        OffsetPageTable::new(&mut *l4_ptr, VirtAddr::zero())
    };

    memory::init(boot_info, &mut page_table);

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

    let mut executor = executor::Executor::new();

    let display_width = boot_info.display_info.width as usize;
    let display_height = boot_info.display_info.height as usize;
    executor.spawn(async move {
        render_clock(framebuffer, display_width, display_height).await;
    });

    x86_64::instructions::interrupts::enable();

    loop {
        x86_64::instructions::hlt();
        executor.tick();
    }
}

async fn render_clock(mut framebuffer: Framebuffer, display_width: usize, display_height: usize) {
    let clock_start_time = rtc::Time::now();
    let clock_start_instant = time::Instant::now();

    loop {
        let dur = clock_start_instant.elapsed();
        let current_time = clock_start_time + rtc::Time::from_seconds(dur.as_secs());
        let time_string = format!("{current_time}");

        let col_count = display_width / framebuffer::font::CHAR_WIDTH;
        let row_count = display_height / framebuffer::font::CHAR_HEIGHT;
        let start_col = col_count - 20; // MM/DD/YYYY HH:MM:SS <- 19 chars
        let row = row_count.saturating_sub(2);

        for (col, ch) in time_string.char_indices() {
            framebuffer.draw_ascii_char(
                ch,
                Color::rgb(0xaa, 0xaa, 0xad),
                Color::rgb(0x2B, 0x2B, 0x33),
                10,
                10,
                start_col + col,
                row,
            );
        }

        executor::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(target_os = "none")]
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
