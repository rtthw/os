//! # Memory Management

use {
    alloc::vec::Vec,
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
        "Initializing heap at {:#x}..{:#x} ({} pages, {:#x} bytes)...",
        heap_region.base,
        heap_region.base + heap_region.size,
        heap_region.size / PAGE_SIZE,
        heap_region.size,
    );

    #[allow(static_mut_refs)]
    unsafe {
        ALLOCATOR.lock().init(heap_region.base, heap_region.size);
    }

    // Make sure the heap allocator actually works.
    initial_heap_test(heap_region.base);

    info!("Heap initialized successfully");
}

#[global_allocator]
static mut ALLOCATOR: LockedHeap = LockedHeap::empty();

fn initial_heap_test(heap_addr: usize) {
    {
        let object_1: Vec<u8> = vec![1, 2, 3];
        let object_1_addr = object_1.as_ptr().addr();

        assert!(object_1_addr == heap_addr);
    }

    let object_2: Vec<u8> = vec![4, 5, 6];
    let object_2_addr = object_2.as_ptr().addr();

    // If object 1 failed to deallocate, then this would fail.
    assert!(object_2_addr == heap_addr);

    let object_3: Vec<u8> = vec![7, 8, 9];
    let object_3_addr = object_3.as_ptr().addr();

    // The heap should start at `heap_addr` and grow upwards, so this object should
    // have a higher address.
    assert!(object_3_addr > heap_addr);
}
