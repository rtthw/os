#![no_std]
#![no_main]

#[macro_use]
extern crate alloc;

use {
    boot_info::BootInfo,
    elf::{ElfFile, ProgramHeaderType},
    log::{info, warn},
    memory_types::PAGE_SIZE,
    uefi::{
        CStr16, Status,
        boot::{self, AllocateType, MemoryType},
        cstr16, entry,
        proto::media::{file::*, fs::SimpleFileSystem},
        system,
        table::cfg::ConfigTableEntry,
    },
};


const KERNEL_PATH: &CStr16 = cstr16!("kernel");

#[entry]
fn main() -> Status {
    uefi::helpers::init().unwrap();
    info!("BOOT");

    let kernel_entry_point_addr = load_kernel();
    info!("Kernel entry point @ {:#x}", kernel_entry_point_addr);

    let rsdp_address = system::with_config_table(|e| {
        let acpi2_entry = e.iter().find(|e| e.guid == ConfigTableEntry::ACPI2_GUID);
        acpi2_entry.map(|e| e.address as u64)
    });
    if let Some(addr) = rsdp_address {
        info!("RSDP @ {addr:#x}");
    } else {
        warn!("RSDP not found");
    }

    info!("Jumping to kernel...");

    let _memory_map = unsafe { boot::exit_boot_services(Some(MemoryType::RUNTIME_SERVICES_DATA)) };
    let boot_info = BootInfo {
        // memory_map,
        rsdp_address,
    };

    jump_to_kernel(kernel_entry_point_addr, &boot_info);

    Status::SUCCESS
}

fn load_kernel() -> u64 {
    let fs = boot::get_handle_for_protocol::<SimpleFileSystem>().unwrap();
    let mut root = boot::open_protocol_exclusive::<SimpleFileSystem>(fs)
        .unwrap()
        .open_volume()
        .unwrap();
    let file_type = root
        .open(KERNEL_PATH, FileMode::Read, FileAttribute::empty())
        .unwrap()
        .into_type()
        .unwrap();
    let mut file = match file_type {
        FileType::Regular(file) => file,
        FileType::Dir(_) => panic!("kernel path does not point to a file"),
    };
    let file_info = file.get_boxed_info::<FileInfo>().unwrap();
    let file_size = file_info.file_size() as usize;

    let mut buf = vec![0; file_size];
    file.read(&mut buf).unwrap();

    let elf = ElfFile::new(&buf).unwrap();

    let mut start_addr = usize::MAX;
    let mut end_addr = 0;

    for program_header in elf.program_iter() {
        if program_header.get_type().unwrap() != ProgramHeaderType::Load {
            continue;
        }

        // info!(
        //     "PH_LOAD @ {:#x}..{:#x}",
        //     ph.virtual_addr,
        //     ph.virtual_addr + ph.mem_size,
        // );

        start_addr = start_addr.min(program_header.virtual_addr as usize);
        end_addr = end_addr.max((program_header.virtual_addr + program_header.mem_size) as usize);
    }

    let page_count = (end_addr - start_addr).div_ceil(PAGE_SIZE);
    boot::allocate_pages(
        AllocateType::Address(start_addr as u64),
        MemoryType::LOADER_DATA,
        page_count,
    )
    .unwrap();

    for program_header in elf.program_iter() {
        if program_header.get_type().unwrap() != ProgramHeaderType::Load {
            continue;
        }

        let addr = program_header.virtual_addr;
        let offset = program_header.offset as usize;
        let size_in_file = program_header.file_size as usize;
        let size_in_memory = program_header.mem_size as usize;

        // info!(
        //     "PH_SIZE @ {:#x}: FILE={:#x}, MEM={:#x}",
        //     offset, size_in_file, size_in_memory
        // );

        let dst = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, size_in_memory) };
        dst[..size_in_file].copy_from_slice(&buf[offset..offset + size_in_file]);
        dst[size_in_file..].fill(0);
    }

    info!(
        "Loaded kernel file @ {:#x}..{:#x} ({} bytes)",
        start_addr, end_addr, file_size,
    );

    elf.header.body.entry_point
}

fn jump_to_kernel(entry_point_addr: u64, boot_info: &BootInfo) {
    let entry_point: extern "sysv64" fn(*const BootInfo) =
        unsafe { core::mem::transmute(entry_point_addr) };
    entry_point(boot_info);
}
