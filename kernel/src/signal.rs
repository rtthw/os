//! # Signal Handling

use crate::{file::File, traits};



#[repr(i32)]
#[non_exhaustive]
pub enum Signal {
    HUP = libc::SIGHUP,
    INT = libc::SIGINT,
    QUIT = libc::SIGQUIT,
    ILL = libc::SIGILL,
    TRAP = libc::SIGTRAP,
    ABRT = libc::SIGABRT, // IOT
    BUS = libc::SIGBUS,
    FPE = libc::SIGFPE,
    KILL = libc::SIGKILL,
    USR1 = libc::SIGUSR1,
    SEGV = libc::SIGSEGV,
    USR2 = libc::SIGUSR2,
    PIPE = libc::SIGPIPE,
    ALRM = libc::SIGALRM,
    TERM = libc::SIGTERM,
    STKFLT = libc::SIGSTKFLT,
    CHLD = libc::SIGCHLD, // CLD
    CONT = libc::SIGCONT,
    STOP = libc::SIGSTOP,
    TSTP = libc::SIGTSTP,
    TTIN = libc::SIGTTIN,
    TTOU = libc::SIGTTOU,
    URG = libc::SIGURG,
    XCPU = libc::SIGXCPU,
    XFSZ = libc::SIGXFSZ,
    VTALRM = libc::SIGVTALRM,
    PROF = libc::SIGPROF,
    WINCH = libc::SIGWINCH,
    IO = libc::SIGIO, // POLL
    PWR = libc::SIGPWR, // INFO
    SYS = libc::SIGSYS, // UNUSED
}

impl TryFrom<i32> for Signal {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        use Signal::*;

        Ok(match value {
            libc::SIGHUP => HUP,
            libc::SIGINT => INT,
            libc::SIGQUIT => QUIT,
            libc::SIGILL => ILL,
            libc::SIGTRAP => TRAP,
            libc::SIGABRT => ABRT,
            libc::SIGBUS => BUS,
            libc::SIGFPE => FPE,
            libc::SIGKILL => KILL,
            libc::SIGUSR1 => USR1,
            libc::SIGSEGV => SEGV,
            libc::SIGUSR2 => USR2,
            libc::SIGPIPE => PIPE,
            libc::SIGALRM => ALRM,
            libc::SIGTERM => TERM,
            libc::SIGSTKFLT => STKFLT,
            libc::SIGCHLD => CHLD,
            libc::SIGCONT => CONT,
            libc::SIGSTOP => STOP,
            libc::SIGTSTP => TSTP,
            libc::SIGTTIN => TTIN,
            libc::SIGTTOU => TTOU,
            libc::SIGURG => URG,
            libc::SIGXCPU => XCPU,
            libc::SIGXFSZ => XFSZ,
            libc::SIGVTALRM => VTALRM,
            libc::SIGPROF => PROF,
            libc::SIGWINCH => WINCH,
            libc::SIGIO => IO,
            libc::SIGPWR => PWR,
            libc::SIGSYS => SYS,
            _ => Err(())?,
        })
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
    pub fn block(&self) -> Result<(), (/* TODO */)> {
        let res = unsafe {
            libc::sigprocmask(
                libc::SIG_BLOCK,
                &self.raw,
                core::ptr::null_mut(),
            )
        };
        if res == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }

    // https://www.man7.org/linux/man-pages/man3/sigwait.3.html
    pub fn wait(&self) -> Result<Signal, (/* TODO */)> {
        let mut sig_ptr = core::mem::MaybeUninit::uninit();
        let res = unsafe { libc::sigwait(&self.raw, sig_ptr.as_mut_ptr()) };
        if res == 0 {
            let num = unsafe { sig_ptr.assume_init() };
            Signal::try_from(num)
        } else {
            todo!("error handling")
        }
    }
}

// https://www.man7.org/linux/man-pages/man3/pthread_sigmask.3.html
impl SignalMask {
    pub fn thread_set_mask(&self) -> Result<(), (/* TODO */)> {
        let res = unsafe {
            libc::pthread_sigmask(
                libc::SIG_SETMASK,
                &self.raw as *const libc::sigset_t,
                core::ptr::null_mut(),
            )
        };

        if res == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }

    pub fn thread_block(&self) -> Result<(), (/* TODO */)> {
        let res = unsafe {
            libc::pthread_sigmask(
                libc::SIG_BLOCK,
                &self.raw as *const libc::sigset_t,
                core::ptr::null_mut(),
            )
        };

        if res == -1 {
            todo!("error handling")
        } else {
            Ok(())
        }
    }

    pub fn thread_unblock(&self) -> Result<(), (/* TODO */)> {
        let res = unsafe {
            libc::pthread_sigmask(
                libc::SIG_UNBLOCK,
                &self.raw as *const libc::sigset_t,
                core::ptr::null_mut(),
            )
        };

        if res == -1 {
            todo!("error handling")
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
    pub fn open(mask: &SignalMask) -> Result<Self, (/* TODO */)> {
        let res = unsafe {
            libc::signalfd(
                -1, // Create a new file descriptor.
                &mask.raw,
                libc::SFD_CLOEXEC,
            )
        };
        if res == -1 {
            todo!("error handling")
        } else {
            Ok(Self { fd: res })
        }
    }

    pub fn open_non_blocking(mask: &SignalMask) -> Result<Self, (/* TODO */)> {
        let res = unsafe {
            libc::signalfd(
                -1, // Create a new file descriptor.
                &mask.raw,
                libc::SFD_CLOEXEC | libc::SFD_NONBLOCK,
            )
        };
        if res == -1 {
            todo!("error handling")
        } else {
            Ok(Self { fd: res })
        }
    }
}
