//! # Advanced Technology Attachment (ATA)

use {
    alloc::{collections::vec_deque::VecDeque, string::String, vec::Vec},
    bit_utils::bit_field,
    core::{fmt, time::Duration},
    log::{debug, info, trace, warn},
    spin_mutex::Mutex,
    x86_64::instructions::port::Port,
};


const SECTOR_SIZE: usize = 512;

const DATA_REGISTER_OFFSET: u16 = 0;
const _ERROR_REGISTER_OFFSET: u16 = 1;
const SECTOR_COUNT_REGISTER_OFFSET: u16 = 2;
const LBA_LOW_REGISTER_OFFSET: u16 = 3;
const LBA_MID_REGISTER_OFFSET: u16 = 4;
const LBA_HIGH_REGISTER_OFFSET: u16 = 5;
const DRIVE_REGISTER_OFFSET: u16 = 6;
const STATUS_REGISTER_OFFSET: u16 = 7;
const COMMAND_REGISTER_OFFSET: u16 = 7;
const ALT_STATUS_REGISTER_OFFSET: u16 = 0;

const FLOATING_BUS_STATUS: u8 = 0xFF;

const READ_COMMAND: u16 = 0x20;
const _WRITE_COMMAND: u16 = 0x30;
const IDENTIFY_COMMAND: u16 = 0xEC;

pub static BUSES: Mutex<[Bus; 2]> =
    Mutex::new([Bus::new(0, 0x1F0, 0x3F6), Bus::new(1, 0x170, 0x376)]);



pub fn init() {
    for mut drive in enumerate_drives() {
        let partitions = get_drive_partitions(&mut drive).unwrap();
        info!(
            "ATA {:x}:{:x} | {drive}\n\
            \tpartitions: {partitions:?}",
            drive.bus, drive.id,
        );

        for partition in partitions {
            let Partition::Boot {
                fs_type,
                lba_start,
                lba_sector_count,
            } = partition;

            assert_eq!(fs_type, FSTYPE_VFAT, "TODO: Support other filesystem types");

            let mut buf = [0; SECTOR_SIZE];
            drive
                .read(lba_start, &mut buf)
                .map_err(|_| "failed to read VFAT boot sector")
                .unwrap();

            let boot_sector: Fat16BootSector = unsafe { core::mem::transmute(buf) };
            debug!("{boot_sector:#?}");

            let mut data_buf = [0; SECTOR_SIZE];
            drive
                .read(
                    lba_start + boot_sector.root_sector_offset() as u32,
                    &mut data_buf,
                )
                .map_err(|_| "failed to read VFAT data sector")
                .unwrap();

            debug!("Listing root directory entries...");

            let entries: [DirectoryEntry; 16] = unsafe { core::mem::transmute(data_buf) };
            let mut current_lfn_buf = VecDeque::new();
            for entry in entries {
                if entry.kind() == EntryKind::Null && entry.attr().is_none() {
                    continue;
                }

                if entry.lfn_index().is_some_and(|i| i >= 1) {
                    if let Some(name) = entry.long_file_name() {
                        current_lfn_buf.push_front(name);
                    }
                    continue;
                }

                if entry
                    .attr()
                    .is_some_and(|attr| matches!(attr, Attribute::Archive | Attribute::Directory))
                {
                    let file_name = if current_lfn_buf.len() > 0 {
                        current_lfn_buf.iter().fold(String::new(), |acc, s| acc + s)
                    } else {
                        entry.short_file_name().unwrap()
                    };
                    current_lfn_buf.clear();

                    debug!(
                        "\t/{file_name} ({} bytes) @ {}",
                        entry.size(),
                        entry.cluster_index(),
                    );
                }
            }
        }
    }
}

pub fn enumerate_drives() -> Vec<Drive> {
    let mut drives = Vec::new();
    for bus in 0..2 {
        for disk in 0..2 {
            if let Some(drive) = Drive::open(bus, disk) {
                drives.push(drive)
            }
        }
    }

    drives
}



#[derive(Clone, Debug)]
pub struct Bus {
    id: u8,
    selected_drive: Option<u8>,

    // https://wiki.osdev.org/ATA_PIO#Registers
    data_register: Port<u16>,
    sector_count_register: Port<u8>,
    lba_low_register: Port<u8>,
    lba_mid_register: Port<u8>,
    lba_high_register: Port<u8>,
    drive_register: Port<u8>,
    status_register: Port<u8>,
    command_register: Port<u8>,
    alt_status_register: Port<u8>,
}

impl Bus {
    const fn new(id: u8, io_port_base: u16, control_port_base: u16) -> Self {
        Self {
            id,
            selected_drive: None,

            // https://wiki.osdev.org/ATA_PIO#Registers
            data_register: Port::new(io_port_base + DATA_REGISTER_OFFSET),
            sector_count_register: Port::new(io_port_base + SECTOR_COUNT_REGISTER_OFFSET),
            lba_low_register: Port::new(io_port_base + LBA_LOW_REGISTER_OFFSET),
            lba_mid_register: Port::new(io_port_base + LBA_MID_REGISTER_OFFSET),
            lba_high_register: Port::new(io_port_base + LBA_HIGH_REGISTER_OFFSET),
            drive_register: Port::new(io_port_base + DRIVE_REGISTER_OFFSET),
            status_register: Port::new(io_port_base + STATUS_REGISTER_OFFSET),
            command_register: Port::new(io_port_base + COMMAND_REGISTER_OFFSET),
            alt_status_register: Port::new(control_port_base + ALT_STATUS_REGISTER_OFFSET),
        }
    }

    fn status(&mut self) -> Status {
        Status(unsafe { self.alt_status_register.read() })
    }

    fn read_data(&mut self) -> u16 {
        unsafe { self.data_register.read() }
    }

    fn read(&mut self, drive: u8, block: u32, buf: &mut [u8]) -> Result<(), &'static str> {
        debug_assert!(buf.len() == SECTOR_SIZE);

        self.select_drive(drive)?;
        self.write_command(drive, block, READ_COMMAND)?;

        for chunk in buf.chunks_mut(2) {
            let data = self.read_data().to_le_bytes();
            chunk.clone_from_slice(&data);
        }

        if self.status().error() {
            debug!("Failed to read ATA bus {}", self.id);

            Err("failed to read bus")
        } else {
            Ok(())
        }
    }

    fn poll(&mut self, status_check: fn(Status) -> bool) -> Result<(), &'static str> {
        let start = time::now();
        while !status_check(self.status()) {
            if time::now().duration_since(start) > Duration::from_secs(1) {
                warn!("ATA bus {} hangup while polling status", self.id);

                return Err("poll hangup");
            }

            core::hint::spin_loop();
        }

        Ok(())
    }

    fn select_drive(&mut self, drive: u8) -> Result<(), &'static str> {
        self.poll(|status| !status.busy())?;
        self.poll(|status| !status.data_request())?;

        if self.selected_drive.is_some_and(|d| d == drive) {
            return Ok(());
        } else {
            self.selected_drive = Some(drive);
        }

        unsafe { self.drive_register.write(0b10100000 | (drive << 4)) }

        let start = time::now();
        while time::now().duration_since(start) < Duration::from_nanos(400) {
            core::hint::spin_loop();
        }

        self.poll(|status| !status.busy())?;
        self.poll(|status| !status.data_request())?;

        Ok(())
    }

    fn write_command(&mut self, drive: u8, block: u32, command: u16) -> Result<(), &'static str> {
        let block_bytes = block.to_le_bytes();
        unsafe {
            self.sector_count_register.write(1);
            self.lba_low_register.write(block_bytes[0]);
            self.lba_mid_register.write(block_bytes[1]);
            self.lba_high_register.write(block_bytes[2]);
            self.drive_register
                .write(block_bytes[3] | (0b11100000 | (drive << 4)));
        }

        unsafe { self.command_register.write(command as u8) }

        let start = time::now();
        while time::now().duration_since(start) < Duration::from_nanos(400) {
            core::hint::spin_loop();
        }

        _ = self.status();
        unsafe {
            _ = self.status_register.read();
        }

        // https://wiki.osdev.org/ATA_PIO#IDENTIFY_command
        if self.status().0 == 0 {
            return Err("drive does not exist");
        }

        if self.status().error() {
            return Err("failed to write command");
        }

        self.poll(|status| !status.busy())?;
        self.poll(|status| status.data_request())?;

        Ok(())
    }

    // https://wiki.osdev.org/ATA_PIO#IDENTIFY_command
    fn identify_drive(&mut self, drive: u8) -> Result<IdentifyResponse, &'static str> {
        // https://wiki.osdev.org/ATA_PIO#Floating_Bus
        if self.status().0 == FLOATING_BUS_STATUS {
            return Ok(IdentifyResponse::None);
        }

        self.select_drive(drive)?;

        // https://wiki.osdev.org/ATA_PIO#%22Command_Aborted%22
        if self.write_command(drive, 0, IDENTIFY_COMMAND).is_err() {
            return Ok(IdentifyResponse::None);
        }

        // https://wiki.osdev.org/ATA_PIO#Detecting_device_types
        match unsafe { (self.lba_mid_register.read(), self.lba_high_register.read()) } {
            (0x00, 0x00) => Ok(IdentifyResponse::Pata([(); 256].map(|_| self.read_data()))),
            (0x14, 0xEB) => Ok(IdentifyResponse::PataPi),
            (0x3C, 0xC3) => Ok(IdentifyResponse::Sata),

            (_, _) => Err("unknown device type"),
        }
    }
}



#[derive(Debug)]
pub struct Drive {
    pub id: u8,
    pub bus: u8,
    model: String,
    serial: String,
    sector_count: u32,
}

impl Drive {
    pub fn open(bus: u8, drive: u8) -> Option<Self> {
        let response = BUSES.lock()[bus as usize].identify_drive(drive);
        match response {
            Ok(IdentifyResponse::Pata(buf)) => {
                let mut serial = String::new();
                let mut model = String::new();

                for word in 10..20 {
                    let value = buf[word];
                    let ch_1 = (value >> 8) as u8 as char;
                    if ch_1 != '\0' {
                        serial.push(ch_1);
                    }
                    let ch_2 = (value as u8) as char;
                    if ch_2 != '\0' {
                        serial.push(ch_2);
                    }
                }
                for word in 27..47 {
                    let value = buf[word];
                    let ch_1 = (value >> 8) as u8 as char;
                    if ch_1 != '\0' {
                        model.push(ch_1);
                    }
                    let ch_2 = (value as u8) as char;
                    if ch_2 != '\0' {
                        model.push(ch_2);
                    }
                }

                let mut sector_count = (buf[100] as u64)
                    | ((buf[101] as u64) << 16)
                    | ((buf[102] as u64) << 32)
                    | ((buf[103] as u64) << 48);

                let _lba_bit_count = if sector_count == 0 {
                    sector_count = (buf[60] as u64) | ((buf[61] as u64) << 16);
                    28
                } else {
                    48
                };

                // TODO: Support different LBA modes.

                Some(Self {
                    id: drive,
                    bus,
                    model: model.trim().into(),
                    serial: serial.trim().into(),
                    sector_count: sector_count as u32,
                })
            }
            Ok(IdentifyResponse::Sata) => {
                warn!("SATA drives are not yet supported");
                None
            }
            Ok(IdentifyResponse::PataPi) => {
                warn!("ATA-PI drives are not yet supported");
                None
            }
            Ok(IdentifyResponse::None) => None,

            Err(error) => {
                warn!("failed to identify ATA {bus:x}:{drive:x}: {error}");
                None
            }
        }
    }

    pub fn read(&mut self, block: u32, buf: &mut [u8]) -> Result<(), &'static str> {
        if block == self.sector_count {
            return Err("attempted to read past end of drive");
        }

        let mut buses = BUSES.lock();
        let bus = &mut buses[self.bus as usize];
        bus.read(self.id, block, buf)?;

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.sector_count as usize * SECTOR_SIZE
    }
}

impl fmt::Display for Drive {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} ({} blocks, {} bytes)",
            self.model,
            self.serial,
            self.sector_count,
            self.len(),
        )
    }
}



enum IdentifyResponse {
    Pata([u16; 256]),
    PataPi,
    Sata,
    None,
}

bit_field! {
    pub struct Status: u8 {
        pub error: bool = 0,
        pub _index: bool = 1,
        pub _corrected: bool = 2,
        pub data_request: bool = 3,
        pub service_request: bool = 4,
        pub drive_fault: bool = 5,
        pub ready: bool = 6,
        pub busy: bool = 7,
    }
}



#[derive(Debug)]
pub enum Partition {
    Boot {
        fs_type: u8,
        lba_start: u32,
        lba_sector_count: u32,
    },
}

fn get_drive_partitions(drive: &mut Drive) -> Result<Vec<Partition>, &'static str> {
    let mut buf = [0; SECTOR_SIZE];
    drive
        .read(0, &mut buf)
        .map_err(|_| "failed to read MBR sector")?;

    if &buf[510..512] != &[0x55, 0xAA] {
        return Err("MBR was not a valid boot sector");
    }

    let mut partitions = Vec::with_capacity(1);

    for entry_chunk in buf[446..510].chunks_exact(16) {
        if entry_chunk.iter().all(|byte| *byte == 0) {
            continue; // Entry is unused.
        }

        // SAFETY: This is just transmuting a 16-byte slice into a slightly more
        //         structured 16-byte slice.
        let entry = unsafe { &*(entry_chunk.as_ptr() as *const PartitionTableEntry) };
        trace!("Partition table entry: {entry:?}");

        if entry.drive_attrs != 0x80 {
            return Err("MBR partition table contained inactive partition");
        }

        let lba_start = u32::from_le_bytes(entry.lba_start_addr);
        let lba_sector_count = u32::from_le_bytes(entry.lba_sector_count);

        // TODO: Support drives with more than just the boot partition.
        if lba_start + lba_sector_count != drive.sector_count {
            return Err("MBR partition table entry should cover the whole drive");
        }

        partitions.push(Partition::Boot {
            fs_type: entry.filesystem_type,
            lba_start,
            lba_sector_count,
        });
    }

    Ok(partitions)
}

const FSTYPE_VFAT: u8 = 0x06;

#[derive(Debug)]
#[repr(C)]
struct PartitionTableEntry {
    drive_attrs: u8,
    _chs_start_addr: [u8; 3],
    filesystem_type: u8,
    _chs_end_addr: [u8; 3],
    lba_start_addr: [u8; 4],
    lba_sector_count: [u8; 4],
}

#[repr(C)]
struct Fat16BootSector {
    jmp_boot: [u8; 3],
    oem_name: [u8; 8],

    bytes_per_sector: [u8; 2],
    sectors_per_cluster: u8,
    reserved_sector_count: [u8; 2],
    fat_count: u8,
    root_entry_count: [u8; 2],
    sector_count_16: [u8; 2],
    media: u8,
    sectors_per_fat_16: [u8; 2],
    sectors_per_track: [u8; 2],
    head_count: [u8; 2],
    hidden_sector_count: [u8; 4],
    sector_count_32: [u8; 4],

    drive_number: u8,
    _reserved: u8,
    sig: u8,
    volume_serial_number: [u8; 4],
    volume_label: [u8; 11],
    fs_type_label: [u8; 8],

    boot_code: [u8; 448],
    signature: [u8; 2],
}

impl fmt::Debug for Fat16BootSector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fat16BootSector")
            .field("oem_name", &self.oem_name())
            .field("volume_label", &self.volume_label())
            .field("fs_type_label", &self.fs_type_label())
            // Ratios.
            .field("bytes_per_sector", &self.bytes_per_sector())
            .field("sectors_per_cluster", &self.sectors_per_cluster())
            .field("sectors_per_track", &self.sectors_per_track())
            .field("sectors_per_fat", &self.sectors_per_fat())
            .field("entries_per_cluster", &self.entries_per_cluster())
            // Counts and offsets.
            .field("sector_count", &self.sector_count())
            .field("cluster_count", &self.cluster_count())
            .field("hidden_sector_count", &self.hidden_sector_count())
            .field("reserved_sector_count", &self.reserved_sector_count())
            .field("data_sector_count", &self.data_sector_count())
            .field("data_sector_offset", &self.data_sector_offset())
            .field("root_sector_offset", &self.root_sector_offset())
            .field("root_sector_count", &self.root_sector_count())
            .field("root_entry_count", &self.root_entry_count())
            .finish()
    }
}

impl Fat16BootSector {
    pub fn oem_name(&self) -> &str {
        core::str::from_utf8(&self.oem_name).unwrap()
    }

    pub fn volume_label(&self) -> &str {
        core::str::from_utf8(&self.volume_label).unwrap()
    }

    pub fn fs_type_label(&self) -> &str {
        core::str::from_utf8(&self.fs_type_label).unwrap()
    }

    pub const fn is_fat32(&self) -> bool {
        self.sector_count_16() == 0
    }

    pub const fn bytes_per_sector(&self) -> usize {
        u16::from_le_bytes(self.bytes_per_sector) as usize
    }

    pub const fn sectors_per_cluster(&self) -> usize {
        self.sectors_per_cluster as usize
    }

    pub const fn entries_per_cluster(&self) -> usize {
        (self.bytes_per_sector() * self.sectors_per_cluster()) / size_of::<DirectoryEntry>()
    }

    pub const fn sectors_per_track(&self) -> usize {
        u16::from_le_bytes(self.sectors_per_track) as usize
    }

    pub const fn sectors_per_fat(&self) -> usize {
        self.sectors_per_fat_16()
    }

    pub const fn sector_count(&self) -> usize {
        if self.is_fat32() {
            self.sector_count_32()
        } else {
            self.sector_count_16()
        }
    }

    const fn sector_count_16(&self) -> usize {
        u16::from_le_bytes(self.sector_count_16) as usize
    }

    const fn sector_count_32(&self) -> usize {
        u32::from_le_bytes(self.sector_count_32) as usize
    }

    pub const fn data_sector_offset(&self) -> usize {
        self.root_sector_offset() + self.root_sector_count()
    }

    pub const fn data_sector_count(&self) -> usize {
        self.sector_count() - self.data_sector_offset()
    }

    pub const fn hidden_sector_count(&self) -> usize {
        u32::from_le_bytes(self.hidden_sector_count) as usize
    }

    pub const fn reserved_sector_count(&self) -> usize {
        u16::from_le_bytes(self.reserved_sector_count) as usize
    }

    pub const fn root_entry_count(&self) -> usize {
        u16::from_le_bytes(self.root_entry_count) as usize
    }

    pub const fn root_sector_offset(&self) -> usize {
        self.reserved_sector_count() + (self.sectors_per_fat_16() * self.fat_count_16())
    }

    pub const fn root_sector_count(&self) -> usize {
        (self.root_entry_count() * 32 + self.bytes_per_sector() - 1) / self.bytes_per_sector()
    }

    pub const fn cluster_count(&self) -> usize {
        self.data_sector_count() / self.sectors_per_cluster()
    }

    const fn fat_count_16(&self) -> usize {
        self.fat_count as usize
    }

    const fn sectors_per_fat_16(&self) -> usize {
        u16::from_le_bytes(self.sectors_per_fat_16) as usize
    }
}

#[repr(C)]
pub struct DirectoryEntry([u8; 32]);

impl fmt::Debug for DirectoryEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DirectoryEntry")
            .field("kind", &self.kind())
            .field("size", &self.size())
            .field("attr", &self.attr())
            .field("cluster_index", &self.cluster_index())
            .finish()
    }
}

impl DirectoryEntry {
    pub const fn raw(&self) -> &[u8; 32] {
        &self.0
    }

    pub const fn attr(&self) -> Option<Attribute> {
        match self.raw()[11] {
            0x01 => Some(Attribute::ReadOnly),
            0x02 => Some(Attribute::Hidden),
            0x04 => Some(Attribute::System),
            0x08 => Some(Attribute::VolumeLabel),
            0x0F => Some(Attribute::LongFileName),
            0x10 => Some(Attribute::Directory),
            0x20 => Some(Attribute::Archive),
            0x40 => Some(Attribute::Device),

            _ => None,
        }
    }

    pub const fn kind(&self) -> EntryKind {
        let bytes = self.raw();
        match (bytes[0], bytes[10]) {
            (0x00, _) => EntryKind::Null,
            (0xE5, _) => EntryKind::Unused,
            (_, 0x0F) => EntryKind::LongFileName,

            _ => EntryKind::Data,
        }
    }

    pub const fn cluster_index(&self) -> usize {
        let bytes = self.raw();
        // let hi = u16::from_le_bytes([bytes[20], bytes[21]]);
        let lo = u16::from_le_bytes([bytes[26], bytes[27]]);

        // (((hi as u32) << 16) | (lo as u32)) as usize
        lo as usize
    }

    pub const fn size(&self) -> usize {
        let bytes = self.raw();
        u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]) as usize
    }

    // https://wiki.osdev.org/FAT#Long_File_Names
    // https://en.wikipedia.org/wiki/Design_of_the_FAT_file_system#VFAT_long_file_names
    pub fn long_file_name(&self) -> Option<String> {
        if self.attr() != Some(Attribute::LongFileName) {
            return None;
        }

        let bytes = self.raw();
        let mut utf16_buf = Vec::new();

        // The first 5 characters.
        for i in (1..11).step_by(2) {
            if utf16_buf.iter().any(|ch| *ch == 0) {
                break;
            }

            utf16_buf.push(bytes[i] as u16 | bytes[i + 1] as u16);
        }

        // The next 6 characters.
        for i in (14..26).step_by(2) {
            if utf16_buf.iter().any(|ch| *ch == 0) {
                break;
            }

            utf16_buf.push(bytes[i] as u16 | bytes[i + 1] as u16);
        }

        // The final 2 characters.
        for i in (28..32).step_by(2) {
            if utf16_buf.iter().any(|ch| *ch == 0) {
                break;
            }

            utf16_buf.push(bytes[i] as u16 | bytes[i + 1] as u16);
        }

        Some(String::from_utf16_lossy(&utf16_buf).replace("\0", ""))
    }

    pub fn short_file_name(&self) -> Option<String> {
        match self.attr() {
            Some(attr) => match attr {
                Attribute::Archive | Attribute::Directory | Attribute::VolumeLabel => {}

                _ => return None,
            },
            None => return None,
        }

        Some(String::from_utf8_lossy(&self.raw()[0..11]).into_owned())
    }

    fn lfn_index(&self) -> Option<usize> {
        if self.attr() != Some(Attribute::LongFileName) {
            return None;
        }

        Some(self.raw()[0] as usize)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u8)]
pub enum Attribute {
    ReadOnly = 0x01,
    Hidden = 0x02,
    System = 0x04,
    VolumeLabel = 0x08,
    LongFileName = 0x0F,
    Directory = 0x10,
    Archive = 0x20,
    Device = 0x40,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum EntryKind {
    #[default]
    Null,
    Unused,
    LongFileName,
    Data,
}
