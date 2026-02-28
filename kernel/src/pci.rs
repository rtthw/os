//! # Peripheral Component Interconnect (PCI)

use {alloc::vec::Vec, core::fmt::Debug, x86_64::instructions::port::Port};



const VENDOR_RED_HAT: u16 = 0x1AF4;
const VENDOR_INTEL: u16 = 0x8086;

#[derive(Clone)]
pub struct Device {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub device_id: u16,
    pub vendor_id: u16,
    pub class: u16,
    pub header_type: u8,
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

    pub fn bar(&self, slot: u8) -> Option<Bar> {
        if slot >= 6 {
            return None;
        }

        let offset = 16 + slot * 4;
        let bar = unsafe { read(self.bus, self.device, self.function, offset) };

        if !u32_get_bit(bar, 0) {
            let prefetchable = u32_get_bit(bar, 3);
            let address = u32_bit_range(bar, 4, 32) << 4;

            match u32_bit_range(bar, 1, 3) {
                0b00 => {
                    let size = unsafe {
                        write(self.bus, self.device, self.function, offset, 0xffffffff);
                        let readback = read(self.bus, self.device, self.function, offset);
                        write(self.bus, self.device, self.function, offset, address);

                        // BAR is unimplemented.
                        if readback == 0 {
                            return None;
                        }

                        1 << u32_set_range(readback, 0, 4, 0).trailing_zeros()
                    };

                    Some(Bar::Mem32 {
                        address,
                        size,
                        prefetchable,
                    })
                }
                0b10 => {
                    // If we are looking at the last slot, then we can't read a 64-bit value.
                    if slot >= 5 {
                        return None;
                    }

                    let address_upper =
                        unsafe { read(self.bus, self.device, self.function, offset + 4) };

                    let size = unsafe {
                        write(self.bus, self.device, self.function, offset, 0xFFFFFFFF);
                        write(self.bus, self.device, self.function, offset + 4, 0xFFFFFFFF);
                        let mut readback_low = read(self.bus, self.device, self.function, offset);
                        let readback_high = read(self.bus, self.device, self.function, offset + 4);
                        write(self.bus, self.device, self.function, offset, address);
                        write(
                            self.bus,
                            self.device,
                            self.function,
                            offset + 4,
                            address_upper,
                        );

                        readback_low = u32_set_range(readback_low, 0, 4, 0);
                        if readback_low != 0 {
                            (1 << readback_low.trailing_zeros()) as u64
                        } else {
                            1 << ((readback_high.trailing_zeros() + 32) as u64)
                        }
                    };

                    let address = u64_set_range(address as u64, 32, 64, address_upper as u64);

                    Some(Bar::Mem64 {
                        address,
                        size,
                        prefetchable,
                    })
                }

                _ => panic!("unknown PCI BAR memory type"),
            }
        } else {
            Some(Bar::Io {
                port: u32_bit_range(bar, 2, 32) << 2,
            })
        }
    }

    pub unsafe fn read(&self, offset: u8) -> u32 {
        unsafe { read(self.bus, self.device, self.function, offset) }
    }

    pub unsafe fn write(&self, offset: u8, value: u32) {
        unsafe { write(self.bus, self.device, self.function, offset, value) }
    }

    pub unsafe fn read_struct<T: Clone>(&self, offset: u8) -> T {
        let size = size_of::<T>();
        assert_eq!(size % 4, 0);
        let num_words = size / 4;

        let buf: Vec<u32> = (0..num_words)
            .map(|i| {
                let i: u8 = i.try_into().unwrap();
                unsafe { self.read(offset + 4 * i) }
            })
            .collect();

        let ptr = buf.as_ptr() as *const T;

        unsafe { ptr.as_ref().unwrap().clone() }
    }

    pub fn set_msix(&self, enabled: bool) {
        let Some(cap) = self.capabilities().into_iter().find(|cap| cap.id == 0x11) else {
            return;
        };

        let mut word = unsafe { read(self.bus, self.device, self.function, cap.offset) };
        word = *u32_set_bit(&mut word, 31, enabled);

        unsafe { write(self.bus, self.device, self.function, cap.offset, word) };
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
            .field("function", &self.function)
            .field("class", &self.class)
            .field("header_type", &self.header_type)
            .field(
                "bars",
                &(0..6).map(|slot| self.bar(slot)).collect::<Vec<_>>(),
            )
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

#[derive(Clone, Debug)]
pub enum Bar {
    Mem32 {
        address: u32,
        size: u32,
        prefetchable: bool,
    },
    Mem64 {
        address: u64,
        size: u64,
        prefetchable: bool,
    },
    Io {
        port: u32,
    },
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

    let class = unsafe { read(bus, device, function, 0x8) };
    let class = (class >> 16) & 0x0000FFFF;
    let class = class as u16;

    let header_type = unsafe { read(bus, device, function, 0x0C) };
    let header_type = ((header_type >> 16) & 0xFF) as u8;

    let last_row = unsafe { read(bus, device, 0, 0x3C) };

    Some(Device {
        bus,
        device,
        function,
        device_id,
        vendor_id,
        class,
        header_type,
        interrupt_line: (last_row & 0xFF) as u8,
        interrupt_pin: ((last_row >> 8) & 0xFF) as u8,
    })
}

unsafe fn read(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let bus = bus as u32;
    let device = device as u32;
    let function = function as u32;
    let offset = offset as u32;

    let address =
        ((bus << 16) | (device << 11) | (function << 8) | (offset & 0xFC) | 0x80000000) as u32;

    unsafe {
        Port::<u32>::new(0xCF8).write(address);
        Port::<u32>::new(0xCFC).read()
    }
}

unsafe fn write(bus: u8, device: u8, function: u8, offset: u8, value: u32) {
    let bus = bus as u32;
    let device = device as u32;
    let function = function as u32;
    let offset = offset as u32;

    let address =
        ((bus << 16) | (device << 11) | (function << 8) | (offset & 0xfc) | 0x80000000) as u32;

    unsafe {
        Port::<u32>::new(0xCF8).write(address);
        Port::<u32>::new(0xCFC).write(value);
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

const fn u32_get_bit(word: u32, bit: usize) -> bool {
    assert!(bit < 32);
    (word & (1 << bit)) != 0
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

const fn u32_set_range(num: u32, start: usize, end: usize, value: u32) -> u32 {
    if start != end {
        let bitmask: u32 = !(!0 << (32 - end) >> (32 - end) >> start << start);
        (num & bitmask) | (value << start)
    } else {
        num
    }
}

const fn u64_set_range(num: u64, start: usize, end: usize, value: u64) -> u64 {
    if start != end {
        let bitmask: u64 = !(!0 << (64 - end) >> (64 - end) >> start << start);
        (num & bitmask) | (value << start)
    } else {
        num
    }
}
