//! # Virtual I/O Input Device

use {
    crate::{Virtqueue, VirtqueueMessage},
    alloc::vec::Vec,
    core::ops::{Deref, DerefMut},
};


const INPUT_EVENT_SIZE: usize = size_of::<InputEvent>();

pub struct Device {
    virtio_device: crate::Device,
    event_queue: Virtqueue<64, INPUT_EVENT_SIZE>,
}

impl Device {
    /// Create a new VirtIO input device from the given [PCI
    /// device](pci::Device).
    ///
    /// Returns `Err` if the given PCI device is not a VirtIO input device (i.e.
    /// it doesn't have the correct configuration, or isn't an input
    /// device).
    pub fn new(pci_device: pci::Device) -> Result<Self, &'static str> {
        let mut virtio_device = crate::Device::new(pci_device)?;
        let mut event_queue = virtio_device.initialize(0, |dev| dev.initialize_queue(0));

        let msg = [VirtqueueMessage::<InputEvent>::DeviceWrite];
        unsafe { while event_queue.push(&msg).is_ok() {} };

        Ok(Self {
            virtio_device,
            event_queue,
        })
    }

    pub fn poll(&mut self) -> Vec<InputEvent> {
        let mut out = Vec::new();

        while let Some(resp_list) = unsafe { self.event_queue.pop::<1, _>() } {
            let event = resp_list.into_iter().next().unwrap();
            out.push(event.unwrap());

            unsafe {
                self.event_queue
                    .push(&[VirtqueueMessage::<InputEvent>::DeviceWrite])
                    .unwrap();
            }
        }

        out
    }
}

impl Deref for Device {
    type Target = crate::Device;

    fn deref(&self) -> &Self::Target {
        &self.virtio_device
    }
}

impl DerefMut for Device {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.virtio_device
    }
}

#[derive(Clone, Debug, Default)]
#[repr(C)]
pub struct InputEvent {
    pub type_: u16,
    pub code: u16,
    pub value: u32,
}
