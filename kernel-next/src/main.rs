#![no_std]
#![no_main]

mod serial;

use {core::arch::asm, log::info};



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
    log::set_max_level(log::LevelFilter::Trace);
    log::set_logger(&serial::SerialLogger).unwrap();

    info!("KERNEL");
    info!("{boot_info:?}");

    unimplemented!();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{info}");
    loop {}
}

#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub rsdp_address: Option<u64>,
}



const KERNEL_STACK_SIZE: usize = 0x100000;
static KERNEL_STACK: KernelStack = KernelStack::new();

#[repr(align(16))]
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
