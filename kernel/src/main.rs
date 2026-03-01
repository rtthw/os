#![no_main]
#![no_std]

#[macro_use]
extern crate alloc;

mod allocator;
mod input;
mod pci;
mod serial;
mod virtio;
mod virtio_gpu;
mod virtio_input;

use {
    crate::virtio_gpu::Color,
    alloc::vec::Vec,
    log::{debug, info, trace, warn},
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

    let mut pci_devices = pci::enumerate_devices();

    let mut virtio_gpu = {
        let index = pci_devices
            .iter()
            .position(|dev| dev.vendor_id == 0x1af4 && dev.device_id == 0x1050)
            .expect("failed to find VirtIO GPU");
        let pci_device = pci_devices.swap_remove(index);
        virtio_gpu::Device::new(pci_device)
    };

    let display_mode = virtio_gpu.active_display_mode().unwrap();
    trace!("DISPLAY_MODE: {display_mode:#?}");

    let mut framebuffer = virtio_gpu::Framebuffer::new(&display_mode);
    virtio_gpu.initialize_framebuffer(&mut framebuffer);
    virtio_gpu.flush(&mut framebuffer);

    let mut virtio_inputs = pci_devices
        .into_iter()
        .filter(|dev| dev.vendor_id == 0x1af4 && dev.device_id == 0x1040 + 18)
        .map(|pci_device| virtio_input::Device::new(pci_device))
        .collect::<Vec<_>>();

    info!("Starting main loop...");

    let mut mouse_x = framebuffer.width() / 2;
    let mut mouse_y = framebuffer.height() / 2;

    'main_loop: loop {
        for input_device in virtio_inputs.iter_mut() {
            for input_event in input_device.poll() {
                match input_event.type_ {
                    // Just ignore sync events for now.
                    input::EV_SYN => {
                        continue;
                    }
                    input::EV_KEY => match input_event.code {
                        input::KEY_ESC => {
                            break 'main_loop;
                        }
                        _ => {
                            if input_event.value == 0 {
                                trace!("KEY_PRESS: code = {}", input_event.code);
                            }
                        }
                    },
                    input::EV_REL => match input_event.code {
                        input::REL_X => {
                            let delta = input_event.value as i32;
                            mouse_x = 0
                                .max((framebuffer.width() as i32 - 1).min(mouse_x as i32 + delta))
                                as u32;
                        }
                        input::REL_Y => {
                            let delta = input_event.value as i32;
                            mouse_y = 0
                                .max((framebuffer.height() as i32 - 1).min(mouse_y as i32 + delta))
                                as u32;
                        }
                        input::REL_WHEEL => {
                            let delta = input_event.value as i32;
                            trace!("MOUSE_WHEEL: delta = {delta}");
                        }

                        _ => warn!("Unhandled pointer event code {}", input_event.code),
                    },

                    other => {
                        warn!("Unhandled input event: {other:?}");
                    }
                }
            }
        }

        {
            let mut pixels = framebuffer.pixels();
            pixels.fill(Color::gray(0x11));
            for x in mouse_x..(mouse_x + 5) {
                pixels.get_mut(x, mouse_y).map(|c| *c = Color::WHITE);
            }
            for y in (mouse_y + 1)..(mouse_y + 7) {
                pixels.get_mut(mouse_x, y).map(|c| *c = Color::WHITE);
            }
        }

        virtio_gpu.flush(&mut framebuffer);
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
