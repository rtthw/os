//! # Peripheral Component Interconnect (PCI)

use {alloc::vec::Vec, core::fmt::Debug, x86_64::instructions::port::Port};



const VENDOR_RED_HAT: u16 = 0x1AF4;
const VENDOR_INTEL: u16 = 0x8086;

#[derive(Clone)]
pub struct Device {
    pub device: u8,
    pub bus: u8,
    pub device_id: u16,
    pub vendor_id: u16,
    pub class: u16,
    pub header_type: u8,
    pub bars: [u32; 6],
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
}

impl Device {
    pub fn name(&self) -> &'static str {
        match (self.vendor_id, self.device_id) {
            (VENDOR_RED_HAT, red_hat_device) => match red_hat_device {
                0x1000 => "Virtio network device",
                0x1001 => "Virtio block device",
                0x1041 => "Virtio 1.0 network device",
                0x1042 => "Virtio 1.0 block device",
                0x1043 => "Virtio 1.0 console",
                0x1044 => "Virtio 1.0 RNG",
                0x1050 => "Virtio 1.0 GPU",
                0x1051 => "Virtio 1.0 clock/timer",
                0x1052 => "Virtio 1.0 input",

                _ => "Unknown Red Hat, Inc. device",
            },
            (VENDOR_INTEL, intel_device) => match intel_device {
                0x100E => "82540EM Gigabit Ethernet Controller",
                0x1237 => "82441FX PMC [Natoma]",
                0x7000 => "82371SB PIIX3 ISA [Natoma/Triton II]",

                _ => "Unknown Intel Corp. device",
            },

            (_, _) => "Unknown device",
        }
    }

    pub fn capabilities(&self) -> Vec<Capability> {
        get_capabilities(self.bus, self.device, 0)
    }
}

impl Debug for Device {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(&format!("#{} '{}'", self.device, self.name()))
            .field("id", &self.device_id)
            .field(
                "vendor",
                match &self.vendor_id {
                    &VENDOR_RED_HAT => &"Red Hat, Inc.",
                    &VENDOR_INTEL => &"Intel Corp." as &dyn Debug,
                    other => &*other as &dyn Debug,
                },
            )
            .field("bus", &self.bus)
            .field("class", &self.class)
            .field("header_type", &self.header_type)
            .field("bars", &self.bars)
            .field("interrupt_line", &self.interrupt_line)
            .field("interrupt_pin", &self.interrupt_pin)
            .field("capabilities", &self.capabilities())
            .finish()
    }
}

#[derive(Clone)]
pub struct Capability {
    pub id: u8,
    pub offset: u8,
}

impl Debug for Capability {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("{} @ {:#x}", self.id, self.offset))
    }
}

pub fn enumerate_devices() -> Vec<Device> {
    let mut devices = vec![];
    for bus in 0..=255 {
        for id in 0..32 {
            if let Some(device) = get_device(bus, id) {
                devices.push(device);
            }
        }
    }

    devices
}

fn get_device(bus: u8, device: u8) -> Option<Device> {
    let function = 0;

    let (device_id, vendor_id) = get_ids(bus, device, function);
    if vendor_id == 0xFFFF {
        return None;
    }

    let class = unsafe { read(bus, device, 0, 0x8) };
    let class = (class >> 16) & 0x0000FFFF;
    let class = class as u16;

    let header_type = unsafe { read(bus, device, function, 0x0C) };
    let header_type = ((header_type >> 16) & 0xFF) as u8;

    let mut bars = [0, 0, 0, 0, 0, 0];
    unsafe {
        bars[0] = read(bus, device, 0, 0x10);
        bars[1] = read(bus, device, 0, 0x14);
        bars[2] = read(bus, device, 0, 0x18);
        bars[3] = read(bus, device, 0, 0x1C);
        bars[4] = read(bus, device, 0, 0x20);
        bars[5] = read(bus, device, 0, 0x24);
    }

    let last_row = unsafe { read(bus, device, 0, 0x3C) };

    Some(Device {
        device,
        bus,
        device_id,
        vendor_id,
        class,
        header_type,
        bars,
        interrupt_line: (last_row & 0xFF) as u8,
        interrupt_pin: ((last_row >> 8) & 0xFF) as u8,
    })
}

unsafe fn read(bus: u8, device: u8, func: u8, offset: u8) -> u32 {
    let bus = bus as u32;
    let device = device as u32;
    let func = func as u32;
    let offset = offset as u32;

    let address =
        ((bus << 16) | (device << 11) | (func << 8) | (offset & 0xFC) | 0x80000000) as u32;

    unsafe {
        Port::<u32>::new(0xCF8).write(address);
        Port::<u32>::new(0xCFC).read()
    }
}

fn get_ids(bus: u8, device: u8, function: u8) -> (u16, u16) {
    let value = unsafe { read(bus, device, function, 0) };
    let device_id = ((value >> 16) & 0xFFFF) as u16;
    let vendor_id = (value & 0xFFFF) as u16;

    (device_id, vendor_id)
}

fn get_capabilities(bus: u8, device: u8, function: u8) -> Vec<Capability> {
    let mut offset = {
        let mut word = unsafe { read(bus, device, function, 0x34) };
        word = *u32_set_bit(u32_set_bit(&mut word, 0, false), 1, false);
        u32_bit_range(word, 0, 8) as u8
    };

    let mut capabilities = Vec::new();
    while offset != 0 {
        let word = unsafe { read(bus, device, function, offset) };
        let id = u32_bit_range(word, 0, 8) as u8;
        capabilities.push(Capability { id, offset });
        offset = u32_bit_range(word, 8, 16) as u8;
    }

    capabilities
}

const fn u32_bit_range(word: u32, start: usize, end: usize) -> u32 {
    assert!(start != end);
    let bits = word << (32 - end) >> (32 - end);
    bits >> start
}

const fn u32_set_bit(word: &mut u32, bit: usize, value: bool) -> &mut u32 {
    assert!(bit < 32);

    if value {
        *word |= 1 << bit;
    } else {
        *word &= !(1 << bit);
    }

    word
}
