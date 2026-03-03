//! # Advanced Configuration and Power Interface (ACPI)

use {
    crate::get_memory_mapper,
    acpi::AcpiTables,
    core::ptr::NonNull,
    log::{info, warn},
    x86_64::PhysAddr,
};



static mut PM1A_CNT_BLK: u32 = 0;

pub fn setup(rsdp_address: usize) {
    match unsafe { AcpiTables::from_rsdp(AcpiHandler, rsdp_address) } {
        Ok(tables) => {
            if let Some(fadt) = tables.find_table::<acpi::sdt::fadt::Fadt>() {
                if let Ok(block) = fadt.pm1a_control_block() {
                    unsafe {
                        PM1A_CNT_BLK = block.address as u32;
                    }
                }
            }
            if let Ok(info) = acpi::platform::AcpiPlatform::new(tables, AcpiHandler) {
                if let Some(info) = info.processor_info {
                    log_processor_info(&info.boot_processor);
                    for processor in info.application_processors.iter() {
                        log_processor_info(processor);
                    }
                }
            }
        }
        Err(_) => {
            warn!("Could not find ACPI tables for RDSP @ {rsdp_address:#x}");
        }
    };
}

#[derive(Clone, Copy)]
struct AcpiHandler;

impl acpi::Handler for AcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> acpi::PhysicalMapping<Self, T> {
        let mapper = get_memory_mapper();
        let phys_addr = PhysAddr::new(physical_address as u64);
        let virt_addr = mapper.physical_to_virtual(phys_addr);
        let ptr = NonNull::new(virt_addr.as_mut_ptr()).unwrap();

        acpi::PhysicalMapping {
            physical_start: physical_address,
            virtual_start: ptr,
            region_length: size,
            mapped_length: size,
            handler: Self,
        }
    }

    fn unmap_physical_region<T>(_region: &acpi::PhysicalMapping<Self, T>) {}

    fn read_u8(&self, address: usize) -> u8 {
        read_addr::<u8>(address)
    }

    fn read_u16(&self, address: usize) -> u16 {
        read_addr::<u16>(address)
    }

    fn read_u32(&self, address: usize) -> u32 {
        read_addr::<u32>(address)
    }

    fn read_u64(&self, address: usize) -> u64 {
        read_addr::<u64>(address)
    }

    fn write_u8(&self, _address: usize, _value: u8) {
        unimplemented!()
    }

    fn write_u16(&self, _address: usize, _value: u16) {
        unimplemented!()
    }

    fn write_u32(&self, _address: usize, _value: u32) {
        unimplemented!()
    }

    fn write_u64(&self, _address: usize, _value: u64) {
        unimplemented!()
    }

    fn read_io_u8(&self, _port: u16) -> u8 {
        unimplemented!()
    }

    fn read_io_u16(&self, _port: u16) -> u16 {
        unimplemented!()
    }

    fn read_io_u32(&self, _port: u16) -> u32 {
        unimplemented!()
    }

    fn write_io_u8(&self, _port: u16, _value: u8) {
        unimplemented!()
    }

    fn write_io_u16(&self, _port: u16, _value: u16) {
        unimplemented!()
    }

    fn write_io_u32(&self, _port: u16, _value: u32) {
        unimplemented!()
    }

    fn read_pci_u8(&self, _address: acpi::PciAddress, _offset: u16) -> u8 {
        unimplemented!()
    }

    fn read_pci_u16(&self, _address: acpi::PciAddress, _offset: u16) -> u16 {
        unimplemented!()
    }

    fn read_pci_u32(&self, _address: acpi::PciAddress, _offset: u16) -> u32 {
        unimplemented!()
    }

    fn write_pci_u8(&self, _address: acpi::PciAddress, _offset: u16, _value: u8) {
        unimplemented!()
    }

    fn write_pci_u16(&self, _address: acpi::PciAddress, _offset: u16, _value: u16) {
        unimplemented!()
    }

    fn write_pci_u32(&self, _address: acpi::PciAddress, _offset: u16, _value: u32) {
        unimplemented!()
    }

    fn nanos_since_boot(&self) -> u64 {
        unimplemented!()
    }

    fn stall(&self, _microseconds: u64) {
        unimplemented!()
    }

    fn sleep(&self, _milliseconds: u64) {
        unimplemented!()
    }

    fn create_mutex(&self) -> acpi::Handle {
        unimplemented!()
    }

    fn acquire(&self, _mutex: acpi::Handle, _timeout: u16) -> Result<(), acpi::aml::AmlError> {
        unimplemented!()
    }

    fn release(&self, _mutex: acpi::Handle) {
        unimplemented!()
    }
}

fn read_addr<T: Copy>(addr: usize) -> T {
    let virtual_address = get_memory_mapper().physical_to_virtual(PhysAddr::new(addr as u64));
    unsafe { *virtual_address.as_ptr::<T>() }
}

fn log_processor_info(processor: &acpi::platform::Processor) {
    let kind = if processor.is_ap { "AP" } else { "BP" };
    let state = match processor.state {
        acpi::platform::ProcessorState::Disabled => "disabled",
        acpi::platform::ProcessorState::Running => "running",
        acpi::platform::ProcessorState::WaitingForSipi => "waiting",
    };
    info!("CPU #{} = {}, {}", processor.processor_uid, kind, state);
}
