#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

#[macro_use]
extern crate alloc;

mod acpi;
mod apic;
mod gdt;
mod idt;
mod memory;
mod rtc;
mod serial;

use {boot_info::BootInfo, core::arch::asm, log::info, memory_types::MEBIBYTE};



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

    log::set_max_level(log::LevelFilter::Trace);
    log::set_logger(&serial::SerialLogger).unwrap();

    info!("KERNEL STARTUP @ {startup_time}");

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

    x86_64::instructions::interrupts::enable();

    info!("STARTUP SUCCESSFUL");

    loop {
        x86_64::instructions::hlt();
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
