//! # Memory Management

use {
    boot_info::{BootInfo, MemoryRegionKind},
    linked_list_allocator::LockedHeap,
    log::{debug, info},
    memory_types::PAGE_SIZE,
};



pub fn init(boot_info: &BootInfo) {
    info!("Initializing memory management...");

    let heap_region = boot_info
        .memory_map
        .iter()
        .filter(|region| region.kind == MemoryRegionKind::Free)
        .max_by_key(|region| region.size)
        .expect("no suitable memory region available for heap");

    debug!(
        "Initializing heap at {:#x} ({} pages, {:#x} bytes)...",
        heap_region.base,
        heap_region.size / PAGE_SIZE,
        heap_region.size,
    );

    #[allow(static_mut_refs)]
    unsafe {
        ALLOCATOR.lock().init(heap_region.base, heap_region.size);
    }
}

#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::empty();
