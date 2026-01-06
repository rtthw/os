//! # Input Handling


use std::os::fd::AsRawFd as _;

use anyhow::Result;
use kernel::{epoll::{Event, EventPoll}, file::File};

use crate::{EventResponse, EventSource, Shell};



pub struct InputSource {
    device: evdev::Device,
}

impl InputSource {
    pub fn new(device: evdev::Device) -> Result<Self> {
        device.set_nonblocking(true)?;
        Ok(Self {
            device,
        })
    }
}

impl EventSource<Shell> for InputSource {
    type Event = evdev::InputEvent;

    fn init(&mut self, poll: &EventPoll, key: u64) -> Result<()> {
        poll.add(
            &unsafe { File::from_raw(self.device.as_raw_fd()) },
            Event::new(key, true, false),
        )?;

        Ok(())
    }

    fn handle_event<F>(
        &mut self,
        shell: &mut Shell,
        event: Event,
        mut callback: F,
    ) -> Result<EventResponse>
    where
        F: FnMut(&mut Shell, evdev::InputEvent) -> Result<()>,
    {
        if !event.readable() {
            return Ok(EventResponse::Continue);
        }

        for event in self.device.fetch_events()? {
            callback(shell, event)?;
        }

        Ok(EventResponse::Continue)
    }

    fn cleanup(&mut self, poll: &EventPoll) -> Result<()> {
        poll.remove(&unsafe { File::from_raw(self.device.as_raw_fd()) })?;
        Ok(())
    }
}
