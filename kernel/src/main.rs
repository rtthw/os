#![no_main]
#![no_std]

#[macro_use]
extern crate alloc;

mod allocator;
mod pci;
mod serial;

use {
    log::{debug, info, trace},
    spin::Once,
    uefi::{mem::memory_map::MemoryMap as _, prelude::*},
    x86_64::{
        PhysAddr, VirtAddr,
        structures::paging::{OffsetPageTable, PageTable, Translate as _, mapper::TranslateResult},
    },
};



#[entry]
fn main() -> Status {
    log::set_max_level(log::LevelFilter::Trace);
    log::set_logger(&serial::SerialLogger).unwrap();

    uefi::helpers::init().unwrap();
    let memory_map = unsafe { boot::exit_boot_services(Some(boot::MemoryType::LOADER_DATA)) };

    info!("Creating memory allocator...");

    // Initialize the memory mapper.
    let _ = get_memory_mapper();

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

    for pci_device in pci::enumerate_devices() {
        trace!("PCI Device: {pci_device:?}");
    }

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



static MEMORY_MAPPER: Once<MemoryMapper> = Once::new();

pub struct MemoryMapper {
    page_table: OffsetPageTable<'static>,
}

impl MemoryMapper {
    /// Convert the given virtual memory address into its physical counterpart.
    pub fn virtual_to_physical(&self, addr: VirtAddr) -> PhysAddr {
        let (frame, offset) = match self.page_table.translate(addr) {
            TranslateResult::Mapped { frame, offset, .. } => (frame, offset),
            TranslateResult::NotMapped => {
                panic!("failed to translate page: virtual address is not physically mapped")
            }
            TranslateResult::InvalidFrameAddress(addr) => {
                panic!("failed to translate page: provided invalid address {addr:#x}")
            }
        };

        frame.start_address() + offset
    }

    /// Convert the given physical memory address into its virtual counterpart.
    pub fn physical_to_virtual(&self, addr: PhysAddr) -> VirtAddr {
        self.page_table.phys_offset() + addr.as_u64()
    }
}

pub fn get_memory_mapper() -> &'static MemoryMapper {
    MEMORY_MAPPER.call_once(|| {
        let physical_offset = VirtAddr::new(0x0);

        // Get the active level 4 table.
        let l4_table = unsafe {
            use x86_64::registers::control::Cr3;
            let (l4_frame, _) = Cr3::read();

            let physical_addr = l4_frame.start_address();
            let virtual_addr = physical_offset + physical_addr.as_u64();
            let ptr: *mut PageTable = virtual_addr.as_mut_ptr();

            &mut *ptr
        };

        let page_table = unsafe { OffsetPageTable::new(l4_table, physical_offset) };

        MemoryMapper { page_table }
    })
}
