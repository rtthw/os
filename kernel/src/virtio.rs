//! # Virtual I/O Devices (VIRTIO)
//!
//! See the [VirtIO 1.3 specification] for more information.
//!
//! [VirtIO 1.3 specification]: https://docs.oasis-open.org/virtio/virtio/v1.3/virtio-v1.3.pdf

use core::ptr::{read_volatile, write_volatile};

use {
    alloc::boxed::Box,
    x86_64::{PhysAddr, VirtAddr},
};

use crate::{get_memory_mapper, pci};

const VIRTIO_F_VERSION_1: u32 = 0x1;

pub const DEVICE_STATUS_RESET: u8 = 0;
pub const DEVICE_STATUS_ACKNOWLEDGE: u8 = 1;
pub const DEVICE_STATUS_DRIVER: u8 = 2;
pub const DEVICE_STATUS_DRIVER_OK: u8 = 4;
pub const DEVICE_STATUS_FEATURES_OK: u8 = 8;
pub const DEVICE_STATUS_NEEDS_RESET: u8 = 64;
pub const DEVICE_STATUS_FAILED: u8 = 128;

pub struct Device {
    pci_device: pci::Device,
    common_config_cap: VirtioCapability,
    notification_cap: VirtioCapability,
    device_specific_config_cap: Option<VirtioCapability>,
    pub common_config: &'static mut VirtioPciCommonCfg,
}

impl Device {
    pub fn new(pci_device: pci::Device) -> Self {
        let find_capability = |cfg_type: u8| -> Option<VirtioCapability> {
            pci_device
                .capabilities()
                .into_iter()
                .filter(|pci_cap| pci_cap.id == 0x09)
                .filter_map(|pci_cap| {
                    let virtio_cap =
                        unsafe { pci_device.read_struct::<VirtioPciCap>(pci_cap.offset) };

                    if virtio_cap.cfg_type != cfg_type {
                        return None;
                    }

                    Some(VirtioCapability {
                        config_space_offset: pci_cap.offset,
                        virtio_cap,
                    })
                })
                .next()
        };

        let common_config_cap = find_capability(VIRTIO_PCI_CAP_COMMON_CFG)
            .expect("failed to find common config capability");
        let notification_cap = find_capability(VIRTIO_PCI_CAP_NOTIFY_CFG)
            .expect("failed to find notification capability");
        let device_specific_config_cap = find_capability(VIRTIO_PCI_CAP_DEVICE_CFG);

        let common_config = {
            let addr = addr_in_bar(&pci_device, &common_config_cap.virtio_cap);
            let ptr = addr.as_mut_ptr() as *mut VirtioPciCommonCfg;
            unsafe { ptr.as_mut().unwrap() }
        };

        Self {
            pci_device,
            common_config_cap,
            notification_cap,
            device_specific_config_cap,
            common_config,
        }
    }

    pub fn initialize<R>(
        &mut self,
        feature_bits: u32,
        mut setup_fn: impl FnOnce(&mut Self) -> R,
    ) -> R {
        // 1. Reset the device.
        self.write_status(DEVICE_STATUS_RESET);

        // 2. Set the ACKNOWLEDGE status bit: the guest OS has noticed the device
        self.write_status(DEVICE_STATUS_ACKNOWLEDGE);

        // 3. Set the DRIVER status bit: the guest OS knows how to drive the device.
        self.write_status(DEVICE_STATUS_DRIVER);

        // 4. Read device feature bits, and write the subset of feature bits understood
        //    by the OS and driver to the device. During this step the driver MAY read
        //    (but MUST NOT write) the device-specific configuration fields to check
        //    that it can support the device before accepting it.
        self.write_feature_bits(0x0, feature_bits);
        self.write_feature_bits(0x1, VIRTIO_F_VERSION_1);

        // 5. Set the FEATURES_OK status bit. The driver MUST NOT accept new feature
        //    bits after this step.
        self.write_status(DEVICE_STATUS_FEATURES_OK);

        // 6. Re-read device status to ensure the FEATURES_OK bit is still set:
        //    otherwise, the device does not support our subset of features and the
        //    device is unusable.
        let status = self.read_status();
        assert_eq!(status, DEVICE_STATUS_FEATURES_OK);

        // 7. Perform device-specific setup, including discovery of virtqueues for the
        //    device, optional per-bus setup, reading and possibly writing the device’s
        //    virtio configuration space, and population of virtqueues.
        let result = setup_fn(self);

        // 8. Set the DRIVER_OK status bit. At this point the device is “live”.
        self.write_status(DEVICE_STATUS_DRIVER_OK);

        result
    }

    pub fn initialize_queue<const QUEUE_SIZE: usize, const BUFFER_SIZE: usize>(
        &mut self,
        index: u16,
    ) -> Virtqueue<QUEUE_SIZE, BUFFER_SIZE> {
        let mapper = get_memory_mapper();

        let mut storage = Box::new(VirtqueueStorage::new());

        for desc in storage.descriptor_area.0.iter_mut() {
            let buffer = Box::new([0u8; BUFFER_SIZE]);
            let buf_ref = Box::leak(buffer);
            let physical_addr = mapper.virtual_to_physical(VirtAddr::from_ptr(buf_ref));

            unsafe {
                write_volatile(&mut desc.addr, physical_addr.as_u64());
            }
        }

        let desc_area_addr = mapper
            .virtual_to_physical(VirtAddr::from_ptr(storage.descriptor_area.0.as_ref()))
            .as_u64();
        let driver_area_addr = mapper
            .virtual_to_physical(VirtAddr::from_ptr(&storage.driver_area))
            .as_u64();
        let device_area_addr = mapper
            .virtual_to_physical(VirtAddr::from_ptr(&storage.device_area))
            .as_u64();

        unsafe {
            let c = &mut self.common_config;

            write_volatile(&mut c.queue_select, index);
            write_volatile(&mut c.queue_desc, desc_area_addr);
            write_volatile(&mut c.queue_driver, driver_area_addr);
            write_volatile(&mut c.queue_device, device_area_addr);
            write_volatile(&mut c.queue_enable, 1);

            let queue_size = read_volatile(&c.queue_size) as usize;
            assert_eq!(queue_size, QUEUE_SIZE);
        }

        let notify_addr = self.queue_notify_addr(index);

        Virtqueue {
            index,
            storage,
            pop_index: 0,
            notify_addr,
            avail_desc: [true; QUEUE_SIZE],
        }
    }

    pub fn write_status(&mut self, value: u8) {
        unsafe { write_volatile(&mut self.common_config.device_status, value) };
    }

    pub fn read_status(&self) -> u8 {
        unsafe { read_volatile(&self.common_config.device_status) }
    }

    fn write_feature_bits(&mut self, select: u32, value: u32) {
        unsafe {
            write_volatile(&mut self.common_config.driver_feature_select, select);
            write_volatile(&mut self.common_config.driver_feature, value);
        }
    }

    fn queue_notify_addr(&mut self, queue_index: u16) -> VirtAddr {
        let queue_notify_offset = unsafe {
            write_volatile(&mut self.common_config.queue_select, queue_index);
            let offset = read_volatile(&self.common_config.queue_notify_off);
            offset as u64
        };
        let notify_offset_multiplier = unsafe {
            let offset = self.notification_cap.config_space_offset + 4;
            self.pci_device.read(offset) as u64
        };

        let base_addr = addr_in_bar(&self.pci_device, &self.notification_cap.virtio_cap);

        base_addr + queue_notify_offset * notify_offset_multiplier
    }
}



#[derive(Debug)]
pub struct VirtioCapability {
    config_space_offset: u8,
    virtio_cap: VirtioPciCap,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct VirtioPciCommonCfg {
    pub device_feature_select: u32,
    pub device_feature: u32,
    pub driver_feature_select: u32,
    pub driver_feature: u32,

    pub msix_config: u16,
    pub num_queues: u16,

    pub device_status: u8,
    pub config_generation: u8,

    pub queue_select: u16,
    pub queue_size: u16,
    pub queue_msix_vector: u16,
    pub queue_enable: u16,
    pub queue_notify_off: u16,

    pub queue_desc: u64,
    pub queue_driver: u64,
    pub queue_device: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct VirtioPciCap {
    cap_vndr: u8,
    cap_next: u8,
    cap_len: u8,
    cfg_type: u8,
    bar: u8,
    padding: [u8; 3],
    offset: u32,
    length: u32,
}

const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 0x1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 0x2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 0x3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 0x4;
const VIRTIO_PCI_CAP_PCI_CFG: u8 = 0x5;

fn addr_in_bar(pci_device: &pci::Device, virtio_cap: &VirtioPciCap) -> VirtAddr {
    let bar_addr = match pci_device.bar(virtio_cap.bar) {
        Some(pci::Bar::Mem32 { address, .. }) => address as u64,
        Some(pci::Bar::Mem64 { address, .. }) => address,

        _ => unimplemented!("addr_in_bar @ {}", virtio_cap.bar),
    };
    let phys_addr = PhysAddr::new(bar_addr + (virtio_cap.offset as u64));

    get_memory_mapper().physical_to_virtual(phys_addr)
}



pub struct Virtqueue<const QUEUE_SIZE: usize, const BUFFER_SIZE: usize> {
    index: u16,
    storage: Box<VirtqueueStorage<QUEUE_SIZE>>,
    pop_index: usize,
    notify_addr: VirtAddr,
    avail_desc: [bool; QUEUE_SIZE],
}

struct VirtqueueStorage<const SIZE: usize> {
    descriptor_area: VirtqueueDescTable<SIZE>,
    driver_area: VirtqueueAvailableRing<SIZE>,
    device_area: VirtqueueUsedRing<SIZE>,
}

impl<const SIZE: usize> VirtqueueStorage<SIZE> {
    const fn new() -> Self {
        Self {
            descriptor_area: VirtqueueDescTable([VirtqueueDesc::ZERO; SIZE]),
            driver_area: VirtqueueAvailableRing {
                flags: 0,
                idx: 0,
                ring: [0; SIZE],
                used_event: 0,
            },
            device_area: VirtqueueUsedRing {
                flags: 0x0,
                idx: 0,
                ring: [VirtqueueUsedElement::ZERO; SIZE],
                avail_event: 0,
            },
        }
    }
}

#[repr(C, align(16))]
pub struct VirtqueueDescTable<const SIZE: usize>([VirtqueueDesc; SIZE]);

#[derive(Clone, Copy, Debug)]
pub struct VirtqueueDesc {
    /// Address (guest-physical).
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

impl VirtqueueDesc {
    pub const ZERO: Self = Self {
        addr: 0,
        len: 0,
        flags: 0,
        next: 0,
    };
}

#[repr(C, align(2))]
struct VirtqueueAvailableRing<const SIZE: usize> {
    flags: u16,
    idx: u16,
    ring: [u16; SIZE],
    used_event: u16,
}

#[repr(C, align(4))]
struct VirtqueueUsedRing<const SIZE: usize> {
    flags: u16,
    idx: u16,
    ring: [VirtqueueUsedElement; SIZE],
    avail_event: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct VirtqueueUsedElement {
    /// Index of start of used descriptor chain.
    id: u32,
    /// The number of bytes written into the device writable portion of the
    /// buffer described by the descriptor chain.
    len: u32,
}

impl VirtqueueUsedElement {
    const ZERO: Self = Self { id: 0, len: 0 };
}
