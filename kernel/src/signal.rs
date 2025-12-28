//! # Signal Handling

use crate::{Error, Result, file::File, traits};



/// A software interrupt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
#[non_exhaustive]
pub enum Signal {
    HUP = 1,
    INT = 2,
    QUIT = 3,
    ILL = 4,
    TRAP = 5,
    ABRT = 6, // IOT
    BUS = 7,
    FPE = 8,
    KILL = 9,
    USR1 = 10,
    SEGV = 11,
    USR2 = 12,
    PIPE = 13,
    ALRM = 14,
    TERM = 15,
    STKFLT = 16,
    CHLD = 17, // CLD
    CONT = 18,
    STOP = 19,
    TSTP = 20,
    TTIN = 21,
    TTOU = 22,
    URG = 23,
    XCPU = 24,
    XFSZ = 25,
    VTALRM = 26,
    PROF = 27,
    WINCH = 28,
    IO = 29, // POLL
    PWR = 30, // INFO
    SYS = 31, // UNUSED
}

impl Signal {
    /// Create a [`Signal`] from its raw `i32` equivalent.
    ///
    /// Returns [`Error::INVAL`] if `num` does not correspond to a known signal.
    ///
    /// ## Example
    /// ```rust
    /// use kernel::Signal;
    /// assert_eq!(Signal::from_raw(11), Ok(Signal::SEGV));
    /// ```
    pub const fn from_raw(num: i32) -> Result<Self> {
        use Signal::*;

        Ok(match num {
            1 => HUP,
            2 => INT,
            3 => QUIT,
            4 => ILL,
            5 => TRAP,
            6 => ABRT,
            7 => BUS,
            8 => FPE,
            9 => KILL,
            10 => USR1,
            11 => SEGV,
            12 => USR2,
            13 => PIPE,
            14 => ALRM,
            15 => TERM,
            16 => STKFLT,
            17 => CHLD,
            18 => CONT,
            19 => STOP,
            20 => TSTP,
            21 => TTIN,
            22 => TTOU,
            23 => URG,
            24 => XCPU,
            25 => XFSZ,
            26 => VTALRM,
            27 => PROF,
            28 => WINCH,
            29 => IO,
            30 => PWR,
            31 => SYS,
            _ => return Err(Error::INVAL),
        })
    }

    /// Returns a string representation of the signal.
    ///
    /// ## Example
    /// ```rust
    /// use kernel::Signal;
    /// assert_eq!(Signal::STOP.as_str(), "SIGSTOP");
    /// ```
    pub const fn as_str(self) -> &'static str {
        match self {
            Signal::HUP => "SIGHUP",
            Signal::INT => "SIGINT",
            Signal::QUIT => "SIGQUIT",
            Signal::ILL => "SIGILL",
            Signal::TRAP => "SIGTRAP",
            Signal::ABRT => "SIGABRT",
            Signal::BUS => "SIGBUS",
            Signal::FPE => "SIGFPE",
            Signal::KILL => "SIGKILL",
            Signal::USR1 => "SIGUSR1",
            Signal::SEGV => "SIGSEGV",
            Signal::USR2 => "SIGUSR2",
            Signal::PIPE => "SIGPIPE",
            Signal::ALRM => "SIGALRM",
            Signal::TERM => "SIGTERM",
            Signal::STKFLT => "SIGSTKFLT",
            Signal::CHLD => "SIGCHLD",
            Signal::CONT => "SIGCONT",
            Signal::STOP => "SIGSTOP",
            Signal::TSTP => "SIGTSTP",
            Signal::TTIN => "SIGTTIN",
            Signal::TTOU => "SIGTTOU",
            Signal::URG => "SIGURG",
            Signal::XCPU => "SIGXCPU",
            Signal::XFSZ => "SIGXFSZ",
            Signal::VTALRM => "SIGVTALRM",
            Signal::PROF => "SIGPROF",
            Signal::WINCH => "SIGWINCH",
            Signal::IO => "SIGIO",
            Signal::PWR => "SIGPWR",
            Signal::SYS => "SIGSYS",
        }
    }
}

impl TryFrom<i32> for Signal {
    type Error = Error;

    fn try_from(value: i32) -> core::result::Result<Self, Self::Error> {
        Self::from_raw(value)
    }
}



pub struct SignalMask {
    raw: libc::sigset_t,
}

impl SignalMask {
    // https://www.man7.org/linux/man-pages/man3/sigfillset.3.html
    pub fn all() -> Self {
        let mut set = core::mem::MaybeUninit::uninit();
        let _ = unsafe { libc::sigfillset(set.as_mut_ptr()) };

        unsafe{ Self { raw: set.assume_init() } }
    }

    // https://www.man7.org/linux/man-pages/man3/sigemptyset.3.html
    pub fn empty() -> Self {
        let mut set = core::mem::MaybeUninit::uninit();
        let _ = unsafe { libc::sigemptyset(set.as_mut_ptr()) };

        unsafe{ Self { raw: set.assume_init() } }
    }

    // https://www.man7.org/linux/man-pages/man3/sigaddset.3.html
    pub fn add(&mut self, sig: Signal) {
        unsafe { libc::sigaddset(&mut self.raw as *mut libc::sigset_t, sig as i32) };
    }

    // https://www.man7.org/linux/man-pages/man3/sigdelset.3.html
    pub fn remove(&mut self, sig: Signal) {
        unsafe { libc::sigdelset(&mut self.raw as *mut libc::sigset_t, sig as i32) };
    }

    // https://www.man7.org/linux/man-pages/man3/sigemptyset.3.html
    pub fn clear(&mut self) {
        unsafe { libc::sigemptyset(&mut self.raw as *mut libc::sigset_t) };
    }

    // https://www.man7.org/linux/man-pages/man2/sigprocmask.2.html
    pub fn block(&self) -> Result<()> {
        let res = unsafe {
            libc::sigprocmask(
                libc::SIG_BLOCK,
                &self.raw,
                core::ptr::null_mut(),
            )
        };
        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man3/sigwait.3.html
    pub fn wait(&self) -> Result<Signal> {
        let mut sig_ptr = core::mem::MaybeUninit::uninit();
        let res = unsafe { libc::sigwait(&self.raw, sig_ptr.as_mut_ptr()) };
        if res == 0 {
            let num = unsafe { sig_ptr.assume_init() };
            Signal::try_from(num)
        } else {
            Err(Error::latest())
        }
    }
}

// https://www.man7.org/linux/man-pages/man3/pthread_sigmask.3.html
impl SignalMask {
    pub fn thread_set_mask(&self) -> Result<()> {
        let res = unsafe {
            libc::pthread_sigmask(
                libc::SIG_SETMASK,
                &self.raw as *const libc::sigset_t,
                core::ptr::null_mut(),
            )
        };

        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(())
        }
    }

    pub fn thread_block(&self) -> Result<()> {
        let res = unsafe {
            libc::pthread_sigmask(
                libc::SIG_BLOCK,
                &self.raw as *const libc::sigset_t,
                core::ptr::null_mut(),
            )
        };

        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(())
        }
    }

    pub fn thread_unblock(&self) -> Result<()> {
        let res = unsafe {
            libc::pthread_sigmask(
                libc::SIG_UNBLOCK,
                &self.raw as *const libc::sigset_t,
                core::ptr::null_mut(),
            )
        };

        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(())
        }
    }
}



#[derive(Eq, PartialEq)]
#[repr(transparent)]
// https://www.man7.org/linux/man-pages/man2/signalfd.2.html
pub struct SignalFile {
    pub(crate) fd: i32,
}

impl traits::AsFile for SignalFile {
    fn as_file(&self) -> crate::file::File {
        File { fd: self.fd }
    }
}

impl SignalFile {
    pub fn open(mask: &SignalMask) -> Result<Self> {
        let res = unsafe {
            libc::signalfd(
                -1, // Create a new file descriptor.
                &mask.raw,
                libc::SFD_CLOEXEC,
            )
        };
        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(Self { fd: res })
        }
    }

    pub fn open_non_blocking(mask: &SignalMask) -> Result<Self> {
        let res = unsafe {
            libc::signalfd(
                -1, // Create a new file descriptor.
                &mask.raw,
                libc::SFD_CLOEXEC | libc::SFD_NONBLOCK,
            )
        };
        if res == -1 {
            Err(Error::latest())
        } else {
            Ok(Self { fd: res })
        }
    }
}
