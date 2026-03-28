//! # Scheduler

use {
    crate::{KERNEL_STACK, KERNEL_STACK_SIZE, gdt},
    alloc::{
        boxed::Box,
        collections::{btree_map::BTreeMap, vec_deque::VecDeque},
        string::String,
        vec::Vec,
    },
    core::{
        arch::asm,
        fmt,
        pin::Pin,
        sync::atomic::{AtomicU64, Ordering},
    },
    log::{info, warn},
    memory_types::PAGE_SIZE,
    spin_mutex::Mutex,
    x86_64::{
        VirtAddr, instructions::interrupts::without_interrupts, registers::rflags::RFlags,
        structures::idt::InterruptStackFrameValue,
    },
};


const IDLE_WORLD_ID: u64 = 0;
const DEFAULT_STACK_SIZE: usize = PAGE_SIZE * 8;

pub fn run() -> ! {
    SCHEDULER.lock().init();
    schedule()
}

#[macro_export]
macro_rules! define_interrupt_handler_with_preemption {
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

                with_scheduler(|scheduler| scheduler.preempt_current_context(context));

                {
                    $($body)*
                }

                // Finally, we schedule the next world to run. This function never returns.
                schedule()
            }
        }
    };
}

static SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());

pub struct Scheduler {
    current: Option<World>,
    queue: BTreeMap<Priority, VecDeque<World>>,
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

        self.add_to_queue(World {
            id: IDLE_WORLD_ID,
            name: "idle".into(),
            stack: Box::pin(Vec::with_capacity(PAGE_SIZE)),
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

    fn add_to_queue(&mut self, world: World) {
        self.queue
            .entry(world.priority)
            .or_default()
            .push_back(world);
    }

    fn next_ready(&mut self) -> World {
        self.queue
            .values_mut()
            .find(|q| !q.is_empty())
            .expect("should at least have an idle world available")
            .pop_front()
            .unwrap()
    }

    fn schedule_next(&mut self) -> ExecutionContext {
        if self.current.is_none() {
            let world = self.next_ready();

            // TODO: Load the world's address space.

            self.current = Some(world);
        }

        self.current
            .as_mut()
            .expect("current world should exist")
            .context
            .take()
            .expect("current world should have a context")
    }

    pub fn preempt_current_context(&mut self, context: ExecutionContext) {
        let prev_context = self
            .current
            .as_mut()
            .expect("a world should be running")
            .context
            .replace(context);

        assert!(prev_context.is_none());

        if self.queue.is_empty() {
            warn!("Attempted preemption with an empty ready queue");
        } else {
            let world = self
                .current
                .take()
                .expect("current world should be available for preemption");
            self.add_to_queue(world);
        }
    }

    pub fn run_world(
        &mut self,
        name: impl Into<String>,
        entry_point: *const fn() -> !,
        stack_size: Option<usize>,
    ) {
        static WORLD_ID: AtomicU64 = AtomicU64::new(IDLE_WORLD_ID + 1);

        let id = WORLD_ID.fetch_add(1, Ordering::SeqCst);
        let name = name.into();

        let stack_size = stack_size.unwrap_or(DEFAULT_STACK_SIZE);
        let stack = Box::pin(Vec::<u8>::with_capacity(stack_size));

        let context = ExecutionContext {
            registers: CpuRegisters::EMPTY,
            frame: InterruptStackFrameValue::new(
                VirtAddr::from_ptr(entry_point),
                gdt::selectors().kernel_code,
                RFlags::INTERRUPT_FLAG,
                VirtAddr::from_ptr(unsafe { stack.as_ptr().add(stack_size) }),
                gdt::selectors().kernel_data,
            ),
        };

        let world = World {
            id,
            name,
            priority: Priority::Normal,
            stack,
            context: Some(context),
        };

        info!("Running {world}");

        self.add_to_queue(world);
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

define_interrupt_handler_with_preemption!(defer_interrupt_handler {
    // Do nothing, allow preemption.
});

/// Defer execution to the scheduler.
pub fn defer() {
    unsafe {
        core::arch::asm!("int 0x40");
    }
}

#[derive(Debug)]
struct World {
    id: u64,
    name: String,
    priority: Priority,
    stack: Pin<Box<Vec<u8>>>, // TODO: Does this need to be pinned?
    context: Option<ExecutionContext>,
}

impl fmt::Display for World {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("World #{} '{}'", self.id, self.name))
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
