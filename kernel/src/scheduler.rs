//! # Scheduler

use {
    crate::{
        KERNEL_STACK, KERNEL_STACK_SIZE, gdt,
        loader::global_loader,
        memory::{AddressSpace, kernel_address_space},
    },
    alloc::{
        collections::{btree_map::BTreeMap, vec_deque::VecDeque},
        string::String,
        vec::Vec,
    },
    core::{
        arch::asm,
        fmt,
        sync::atomic::{AtomicU64, Ordering},
    },
    log::{info, warn},
    memory_types::PAGE_SIZE,
    spin_mutex::Mutex,
    x86_64::{
        VirtAddr,
        instructions::interrupts::without_interrupts,
        registers::rflags::RFlags,
        structures::{
            idt::InterruptStackFrameValue,
            paging::{Page, PageTableFlags},
        },
    },
};


const IDLE_PROCESS_ID: u64 = 0;
const USER_CODE_ADDR: u64 = 0x4444_0000_0000;
const DEFAULT_STACK_SIZE: usize = PAGE_SIZE * 8;

static PROCESS_ID: AtomicU64 = AtomicU64::new(IDLE_PROCESS_ID + 1);

pub fn run() -> ! {
    SCHEDULER.lock().init();
    schedule()
}

#[macro_export]
macro_rules! define_interrupt_handler_with_context {
    ($name:ident { $($body:tt)* }) => {
        #[unsafe(naked)]
        pub extern "x86-interrupt" fn $name(
            frame: ::x86_64::structures::idt::InterruptStackFrame,
        ) {
            use $crate::scheduler::{schedule, with_scheduler, ExecutionContext};

            ::core::arch::naked_asm!(
                "mov    [rsp - 120], r15",
                "mov    [rsp - 112], r14",
                "mov    [rsp - 104], r13",
                "mov    [rsp - 96], r12",
                "mov    [rsp - 88], r11",
                "mov    [rsp - 80], r10",
                "mov    [rsp - 72], r9",
                "mov    [rsp - 64], r8",
                "mov    [rsp - 56], rsi",
                "mov    [rsp - 48], rdi",
                "mov    [rsp - 40], rbp",
                "mov    [rsp - 32], rdx",
                "mov    [rsp - 24], rcx",
                "mov    [rsp - 16], rbx",
                "mov    [rsp - 8],  rax",

                "lea    rdi, [rsp - 120]",
                "sub    rsp, 120",

                // Now that we've properly assembled the context, we can jump to `__handler`.
                "call   {}",
                sym __handler,
            );

            extern "C" fn __handler(context: ExecutionContext) -> ! {
                assert!(!::x86_64::instructions::interrupts::are_enabled());

                with_scheduler(|scheduler| scheduler.set_current_context(context));

                {
                    $($body)*
                }

                // Finally, we schedule the next process to run. This function never returns.
                schedule()
            }
        }
    };
}

static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

pub struct Scheduler {
    current: Option<Process>,
    queue: BTreeMap<Priority, VecDeque<Process>>,
}

impl Scheduler {
    const fn new() -> Self {
        Self {
            current: None,
            queue: BTreeMap::new(),
        }
    }

    fn init(&mut self) {
        fn __idle_loop() -> ! {
            loop {
                x86_64::instructions::hlt();
            }
        }

        self.add_to_queue(Process {
            id: IDLE_PROCESS_ID,
            name: "idle".into(),
            address_space: AddressSpace::new(None),
            priority: Priority::Idle,
            context: Some(ExecutionContext {
                registers: CpuRegisters::EMPTY,
                frame: InterruptStackFrameValue::new(
                    VirtAddr::from_ptr(__idle_loop as *const fn() -> !),
                    gdt::selectors().kernel_code,
                    RFlags::INTERRUPT_FLAG,
                    VirtAddr::from_ptr(unsafe { KERNEL_STACK.as_ptr().add(KERNEL_STACK_SIZE) }),
                    gdt::selectors().kernel_code,
                ),
            }),
        })
    }

    fn add_to_queue(&mut self, process: Process) {
        self.queue
            .entry(process.priority)
            .or_default()
            .push_back(process);
    }

    fn next_ready(&mut self) -> Process {
        self.queue
            .values_mut()
            .find(|q| !q.is_empty())
            .expect("should at least have an idle process available")
            .pop_front()
            .unwrap()
    }

    fn schedule_next(&mut self) -> ExecutionContext {
        if self.current.is_none() {
            let process = self.next_ready();
            process.address_space.enter();
            self.current = Some(process);
        }

        self.current
            .as_mut()
            .expect("current process should exist")
            .context
            .take()
            .expect("current process should have a context")
    }

    pub fn set_current_context(&mut self, context: ExecutionContext) {
        let prev_context = self
            .current
            .as_mut()
            .expect("a process should be running at this point")
            .context
            .replace(context);

        assert!(prev_context.is_none());
    }

    pub fn preempt_current(&mut self) {
        if self.queue.is_empty() {
            warn!("Attempted preemption with an empty ready queue");
        } else {
            let process = self
                .current
                .take()
                .expect("current process should be available for preemption");
            self.add_to_queue(process);
        }
    }

    pub fn run_process(
        &mut self,
        name: impl Into<String>,
        entry_point: *const fn() -> !,
        stack_size: Option<usize>,
    ) {
        let id = PROCESS_ID.fetch_add(1, Ordering::SeqCst);
        let name = name.into();

        // TODO: Process address spaces shouldn't just inherit the kernel address space.

        let address_space = AddressSpace::new(Some(kernel_address_space()));

        let stack_size = stack_size.unwrap_or(DEFAULT_STACK_SIZE);
        let stack_top_addr = VirtAddr::new(USER_CODE_ADDR);
        {
            let top_page = Page::containing_address(stack_top_addr - 1);
            let bottom_page = Page::containing_address(stack_top_addr - stack_size as u64);
            let stack_pages = Page::range_inclusive(bottom_page, top_page);
            address_space.map_pages(
                stack_pages,
                PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE,
            );
        }

        let context = ExecutionContext {
            registers: CpuRegisters::EMPTY,
            frame: InterruptStackFrameValue::new(
                VirtAddr::from_ptr(entry_point),
                gdt::selectors().kernel_code,
                RFlags::INTERRUPT_FLAG,
                stack_top_addr,
                gdt::selectors().kernel_data,
            ),
        };

        let process = Process {
            id,
            name,
            priority: Priority::Normal,
            address_space,
            context: Some(context),
        };

        info!("Running {process}");

        self.add_to_queue(process);
    }

    pub fn run_program(&mut self, name: impl Into<String>, stack_size: Option<usize>) {
        let id = PROCESS_ID.fetch_add(1, Ordering::SeqCst);
        let name = name.into();

        // TODO: Process address spaces shouldn't just inherit the kernel address space.

        let address_space = AddressSpace::new(Some(kernel_address_space()));
        let user_code_addr = VirtAddr::new(USER_CODE_ADDR);
        let user_code_page = Page::containing_address(user_code_addr);

        let _object = global_loader()
            .load_object(&name, &address_space, user_code_page)
            .unwrap();

        // log::debug!("LOADER: {:#?}", global_loader());

        let entry_point_section = global_loader()
            .get_section_ending_with("main")
            .unwrap()
            .upgrade()
            .unwrap();
        let entry_point = user_code_addr + entry_point_section.mapping_offset as u64;

        let stack_size = stack_size.unwrap_or(DEFAULT_STACK_SIZE);
        let stack_top_addr = VirtAddr::new(USER_CODE_ADDR);
        {
            let top_page = Page::containing_address(stack_top_addr - 1);
            let bottom_page = Page::containing_address(stack_top_addr - stack_size as u64);
            let stack_pages = Page::range_inclusive(bottom_page, top_page);
            address_space.map_pages(
                stack_pages,
                PageTableFlags::PRESENT
                    | PageTableFlags::WRITABLE
                    | PageTableFlags::USER_ACCESSIBLE,
            );
        }

        let context = ExecutionContext {
            registers: CpuRegisters::EMPTY,
            frame: InterruptStackFrameValue::new(
                entry_point,
                gdt::selectors().user_code,
                RFlags::INTERRUPT_FLAG,
                stack_top_addr,
                gdt::selectors().user_data,
            ),
        };

        let process = Process {
            id,
            name,
            priority: Priority::Normal,
            address_space,
            context: Some(context),
        };

        info!("Running {process}");

        self.add_to_queue(process);
    }
}

pub fn with_scheduler<F, R>(op: F) -> R
where
    F: FnOnce(&mut Scheduler) -> R,
{
    without_interrupts(|| {
        let mut universe = SCHEDULER.lock();
        op(&mut *universe)
    })
}

pub fn schedule() -> ! {
    let next_context = with_scheduler(Scheduler::schedule_next);
    unsafe {
        asm!(
            "mov    rsp, {}",
            "add    rsp, 120",
            "mov    r15, [rsp - 120]",
            "mov    r14, [rsp - 112]",
            "mov    r13, [rsp - 104]",
            "mov    r12, [rsp - 96]",
            "mov    r11, [rsp - 88]",
            "mov    r10, [rsp - 80]",
            "mov    r9,  [rsp - 72]",
            "mov    r8,  [rsp - 64]",
            "mov    rsi, [rsp - 56]",
            "mov    rdi, [rsp - 48]",
            "mov    rbp, [rsp - 40]",
            "mov    rdx, [rsp - 32]",
            "mov    rcx, [rsp - 24]",
            "mov    rbx, [rsp - 16]",
            "mov    rax, [rsp - 8]",

            "iretq",

            in(reg) &next_context,
            options(noreturn),
        )
    }
}

pub const DEFER_INTERRUPT_NUMBER: u8 = 0x40; // TODO: Choose a less arbitrary number.
pub const EXIT_INTERRUPT_NUMBER: u8 = 0x41;

define_interrupt_handler_with_context!(defer_interrupt_handler {
    with_scheduler(|scheduler| scheduler.preempt_current());
});

define_interrupt_handler_with_context!(exit_interrupt_handler {
    // Exiting the current process is as simple as dropping it. The process
    // will no longer exist within the run queue, and its allocated frames will
    // be deallocated when the address space is dropped.

    kernel_address_space().enter(); // ???: Is this necessary?
    if let Some(process) = with_scheduler(|scheduler| scheduler.current.take()) {
        info!("Exiting {process}");
    } else {
        unreachable!()
    }

    // Immediately after this block is finished, `Scheduler::schedule_next` is
    // called to set the next execution context.
});

/// Defer execution to the scheduler.
pub fn defer() {
    unsafe {
        core::arch::asm!("int 0x40");
    }
}

/// Exit the current process.
pub fn exit() {
    unsafe {
        core::arch::asm!("int 0x41");
    }
}

#[derive(Debug)]
struct Process {
    id: u64,
    name: String,
    priority: Priority,
    address_space: AddressSpace,
    context: Option<ExecutionContext>,
}

impl fmt::Display for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Process #{} '{}'", self.id, self.name))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Priority {
    Normal = 32,
    Idle = 255,
}

#[derive(Debug)]
#[repr(C)]
pub struct ExecutionContext {
    registers: CpuRegisters,
    frame: InterruptStackFrameValue,
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct CpuRegisters {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rsi: u64,
    rdi: u64,
    rbp: u64,
    rdx: u64,
    rcx: u64,
    rbx: u64,
    rax: u64,
}

impl CpuRegisters {
    const EMPTY: Self = Self {
        r15: 0,
        r14: 0,
        r13: 0,
        r12: 0,
        r11: 0,
        r10: 0,
        r9: 0,
        r8: 0,
        rsi: 0,
        rdi: 0,
        rbp: 0,
        rdx: 0,
        rcx: 0,
        rbx: 0,
        rax: 0,
    };
}
