//! # Polling Interfaces

use alloc::vec::Vec;

use crate::{Error, Result, file::File, traits::AsFile};



// https://www.man7.org/linux/man-pages/man7/epoll.7.html
pub struct EventPoll {
    fd: i32,
}

impl AsFile for EventPoll {
    fn as_file(&self) -> File {
        File { fd: self.fd }
    }
}

impl EventPoll {
    // https://www.man7.org/linux/man-pages/man2/epoll_create.2.html
    pub fn create() -> Result<EventPoll> {
        let res = unsafe { libc::epoll_create1(libc::O_CLOEXEC) };
        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(EventPoll { fd: res })
        }
    }

    // https://www.man7.org/linux/man-pages/man2/epoll_ctl.2.html
    pub fn add(&self, fd: &File, mut ev: Event) -> Result<()> {
        let ptr = &mut ev as *mut Event;
        let res = unsafe {
            libc::epoll_ctl(
                self.fd,
                libc::EPOLL_CTL_ADD,
                fd.fd,
                ptr as *mut libc::epoll_event,
            )
        };
        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man2/epoll_ctl.2.html
    pub fn remove(&self, fd: &File) -> Result<()> {
        let res = unsafe {
            libc::epoll_ctl(
                self.fd,
                libc::EPOLL_CTL_DEL,
                fd.fd,
                core::ptr::null_mut(),
            )
        };
        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man2/epoll_wait.2.html
    pub fn wait(&self, events: &mut Vec<Event>, timeout_ms: i32) -> Result<usize> {
        let slice = events.spare_capacity_mut();
        let res = unsafe {
            libc::epoll_wait(
                self.fd,
                slice.as_mut_ptr().cast(),
                slice.len() as _,
                timeout_ms,
            )
        };
        if res == -1 {
            Err(Error::latest())
        } else {
            let new_events_count = res as usize;
            unsafe { events.set_len(events.len() + new_events_count as usize); }
            Ok(new_events_count)
        }
    }
}



// https://www.man7.org/linux/man-pages/man3/epoll_event.3type.html
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Event {
    raw: libc::epoll_event,
}

impl Event {
    pub fn new(data: u64, readable: bool, writable: bool) -> Self {
        let mut flags = 0;
        if readable {
            flags |= libc::EPOLLIN | libc::EPOLLPRI;
        }
        if writable {
            flags |= libc::EPOLLOUT;
        }

        Self {
            raw: libc::epoll_event {
                events: flags as _,
                u64: data,
            },
        }
    }

    pub fn data(&self) -> u64 {
        self.raw.u64
    }

    pub fn readable(&self) -> bool {
        self.raw.events & (libc::EPOLLIN | libc::EPOLLPRI) as u32 != 0
    }

    pub fn writable(&self) -> bool {
        self.raw.events & libc::EPOLLOUT as u32 != 0
    }
}
