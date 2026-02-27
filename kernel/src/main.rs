#![no_main]
#![no_std]

mod allocator;
mod serial;

use {
    log::{debug, info},
    uefi::{mem::memory_map::MemoryMap as _, prelude::*},
};



#[entry]
fn main() -> Status {
    log::set_max_level(log::LevelFilter::Trace);
    log::set_logger(&serial::SerialLogger).unwrap();

    uefi::helpers::init().unwrap();
    let memory_map = unsafe { boot::exit_boot_services(Some(boot::MemoryType::LOADER_DATA)) };

    info!("Creating memory allocator...");

    let heap_desc = memory_map
        .entries()
        .filter(|desc| desc.ty == boot::MemoryType::CONVENTIONAL)
        .max_by_key(|desc| desc.page_count)
        .expect("no suitable memory region available");
    let heap_addr = heap_desc.phys_start as usize;
    let heap_size = 4096 * heap_desc.page_count as usize;

    debug!(
        "Initializing heap at {:#x} ({} pages, {} bytes)...",
        heap_addr, heap_desc.page_count, heap_size,
    );

    #[allow(static_mut_refs)]
    unsafe {
        allocator::ALLOCATOR.init(heap_addr, heap_size);
    }

    info!("Setting up devices...");

    // TODO

    info!("Starting main loop...");

    loop {
        // TODO
        if false {
            break;
        }
    }

    Status::SUCCESS
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{}", info);
    loop {}
}
