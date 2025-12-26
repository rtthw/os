
use crate::{raw, traits};



#[derive(Eq, PartialEq)]
#[repr(transparent)]
pub struct Process {
    pub(crate) id: i32,
}

impl Process {
    // https://www.man7.org/linux/man-pages/man2/getpid.2.html
    pub fn current() -> Self {
        Self {
            id: raw::getpid(),
        }
    }

    // https://www.man7.org/linux/man-pages/man2/getpid.2.html
    pub fn parent() -> Self {
        Self {
            id: raw::getppid(),
        }
    }

    // https://www.man7.org/linux/man-pages/man2/getsid.2.html
    pub fn session(&self) -> Option<Session> {
        let id = raw::getsid(self.id);
        if id == -1 {
            // NOTE: `getsid` only ever returns `ESRCH` (not found) on Linux.
            //       https://www.man7.org/linux/man-pages/man2/getsid.2.html#ERRORS
            None
        } else {
            Some(Session { id })
        }
    }

    // https://www.man7.org/linux/man-pages/man2/getpgrp.2.html
    pub fn group(&self) -> Option<ProcessGroup> {
        let id = raw::getpgid(self.id);
        if id == -1 {
            // NOTE: `getpgid` only ever returns `ESRCH` (not found) on Linux.
            //       https://www.man7.org/linux/man-pages/man2/getpgrp.2.html#ERRORS
            None
        } else {
            Some(ProcessGroup { id })
        }
    }
}

impl PartialEq<i32> for Process {
    fn eq(&self, other: &i32) -> bool {
        self.id == *other
    }
}



#[derive(Eq, PartialEq)]
#[repr(transparent)]
pub struct Session {
    pub(crate) id: i32,
}

impl traits::AsProcess for Session {
    fn as_process(&self) -> Process {
        Process { id: self.id }
    }
}

impl Session {
    // https://www.man7.org/linux/man-pages/man2/getsid.2.html#DESCRIPTION
    pub fn current() -> Self {
        let id = raw::getsid(0);
        assert!(id != -1, "getsid cannot find the current process?");
        Self { id }
    }
}



#[derive(Eq, PartialEq)]
#[repr(transparent)]
pub struct ProcessGroup {
    pub(crate) id: i32,
}

impl traits::AsProcess for ProcessGroup {
    fn as_process(&self) -> Process {
        Process { id: self.id }
    }
}

impl ProcessGroup {
    // https://www.man7.org/linux/man-pages/man2/getpgrp.2.html
    pub fn current() -> Self {
        Self { id: raw::getpgrp() }
    }

    pub fn leader(&self) -> Process {
        Process { id: self.id }
    }
}



#[derive(Eq, PartialEq)]
#[repr(transparent)]
pub struct Thread {
    pub(crate) id: i32,
}

impl Thread {
    // https://www.man7.org/linux/man-pages/man2/gettid.2.html
    pub fn current() -> Self {
        let id = raw::gettid();
        Self { id }
    }

    pub fn id(&self) -> i32 {
        self.id
    }
}
