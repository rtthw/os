//! # Virtual I/O Input Device

use core::ops::{Deref, DerefMut};

use alloc::vec::Vec;

use crate::{
    pci,
    virtio::{self, VirtqueueMessage},
};


const INPUT_EVENT_SIZE: usize = size_of::<InputEvent>();

pub struct Device {
    virtio_device: virtio::Device,
    event_queue: virtio::Virtqueue<64, INPUT_EVENT_SIZE>,
}

impl Device {
    pub fn new(pci_device: pci::Device) -> Self {
        let mut virtio_device = virtio::Device::new(pci_device);
        let mut event_queue = virtio_device.initialize(0, |dev| dev.initialize_queue(0));

        let msg = [VirtqueueMessage::<InputEvent>::DeviceWrite];
        unsafe { while event_queue.push(&msg).is_ok() {} };

        Self {
            virtio_device,
            event_queue,
        }
    }

    pub fn poll(&mut self) -> Vec<InputEvent> {
        let mut out = Vec::new();

        while let Some(resp_list) = unsafe { self.event_queue.pop::<1, _>() } {
            let event = resp_list.into_iter().next().unwrap();
            out.push(event);

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
    type Target = virtio::Device;

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
