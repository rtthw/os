//! # Process Management

#![no_std]

use spin_mutex::Mutex;



/// Defer execution to the next process in the scheduler's run queue.
///
/// Execution will resume when this process is next scheduled.
pub fn defer() {
    unsafe {
        core::arch::asm!("int 0x40");
    }
}

/// Exit the current process.
pub fn exit(code: i64) -> ! {
    unsafe {
        core::arch::asm!(
            "int 0x41",
            in("rdi") code,
            options(noreturn),
        );
    }
}

/// Translate the given virtual address to its physical counterpart.
pub fn translate_address(addr: usize) -> Result<usize, TranslateAddressError> {
    let mut num: isize = 0;
    unsafe {
        core::arch::asm!(
            "int 0x42",
            inout("rax") num,
            in("rdi") addr,
            options(nostack),
        );
    }

    if num < 0 {
        Err(match num {
            -1 => TranslateAddressError::PermissionDenied,
            -2 => TranslateAddressError::AddressNotMapped,
            _ => unreachable!(),
        })
    } else {
        Ok(num.cast_unsigned())
    }
}

#[derive(Debug)]
pub enum TranslateAddressError {
    PermissionDenied,
    AddressNotMapped,
}

impl core::fmt::Display for TranslateAddressError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                TranslateAddressError::PermissionDenied => "permission denied",
                TranslateAddressError::AddressNotMapped => "address is not mapped",
            },
        )
    }
}

/// The policy used to determine how resources are granted to a process.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum AccessPolicy {
    /// The process has access to all resources.
    ///
    /// No checks are performed when it requests access to a resource. The
    /// resource is granted without blocking the process.
    All,
    /// The process has normal access to resources.
    ///
    /// When it requests access to some resource, it will be blocked until
    /// access is granted (or stopped if it is denied).
    #[default]
    Normal,
    // /// The process has no access to resources.
    // ///
    // /// If it requests access to a resource, the process will be stopped.
    // None,
}

/// The execution priority of a process.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Priority {
    None = 0,
    Normal = 32,
    Idle = 255,
}

/// The maximum number of requests that can be sent to the shell in between calls to its
/// request handler. This is not the maximum number of requests that can be pending at a
/// time.
const SHELL_INPUT_QUEUE_SIZE: usize = 8;

#[repr(C, align(4096))]
pub struct ShellQueue {
    pub input: Mutex<SizedQueue<ShellInput, SHELL_INPUT_QUEUE_SIZE>>,
    pub output: Mutex<SizedQueue<ShellOutput, SHELL_INPUT_QUEUE_SIZE>>,
}

impl ShellQueue {
    pub const fn new() -> Self {
        Self {
            input: Mutex::new(SizedQueue::new()),
            output: Mutex::new(SizedQueue::new()),
        }
    }
}

/// A message sent from the kernel to the shell.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum ShellInput {
    /// A process wants to access some module section.
    AccessModuleRequest {
        addr: usize,
        // pages: PageRange,
        // flags: PageTableFlags,
        process_id: u64,
        process_name: SizedString<32>,
        module_name: SizedString<32>,
        section_name: SizedString<32>,
    },
}

/// A message sent from the shell to the kernel.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub enum ShellOutput {
    ExitProcess { code: u64 },
    AllowModuleAccess { addr: usize, process_id: u64 },
}

impl ShellOutput {
    pub const fn unblocks_process(&self) -> bool {
        matches!(
            self,
            Self::ExitProcess { .. } | Self::AllowModuleAccess { .. }
        )
    }
}

pub struct SizedQueue<T: Sized, const SIZE: usize> {
    inner: [Option<T>; SIZE],
    push_cursor: usize,
    pop_cursor: usize,
}

impl<T: Sized, const SIZE: usize> SizedQueue<T, SIZE> {
    const fn new() -> Self {
        Self {
            inner: [const { None }; SIZE],
            push_cursor: 0,
            pop_cursor: 0,
        }
    }

    pub fn push(&mut self, element: T) {
        self.inner[self.push_cursor] = Some(element);
        self.push_cursor += 1;
        if self.push_cursor >= SIZE {
            self.push_cursor = 0;
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        let element = self.inner[self.pop_cursor].take();
        if element.is_some() {
            self.pop_cursor += 1;
            if self.pop_cursor >= SIZE {
                self.pop_cursor = 0;
            }
        }

        element
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct SizedString<const SIZE: usize> {
    bytes: [u8; SIZE],
}

impl<const SIZE: usize> SizedString<SIZE> {
    pub fn new_truncate(string: &str) -> Self {
        let mut bytes = [0; SIZE];
        let len = string.len().min(SIZE);
        bytes[..len].clone_from_slice(&string.as_bytes()[..len]);

        Self { bytes }
    }

    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.bytes) }.trim_matches('\0')
    }
}

impl<const SIZE: usize> core::fmt::Debug for SizedString<SIZE> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
