//! # Asynchronous Executor

use {
    alloc::{
        boxed::Box,
        collections::{binary_heap::BinaryHeap, btree_map::BTreeMap},
        sync::Arc,
        task::Wake,
    },
    core::{
        pin::Pin,
        sync::atomic::{AtomicU64, Ordering},
        task::{Context, Poll, Waker},
    },
    crossbeam_queue::ArrayQueue,
    spin_mutex::Mutex,
    time::{Duration, Instant},
};



pub struct Task {
    // id: u64,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

pub struct Executor {
    tasks: BTreeMap<u64, Task>,
    task_queue: Arc<ArrayQueue<u64>>,
    waker_cache: BTreeMap<u64, Waker>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(ArrayQueue::new(64)),
            waker_cache: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'static) {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
        self.tasks.insert(
            id,
            Task {
                // id,
                future: Box::pin(future),
            },
        );
        self.task_queue.push(id).expect("too many tasks in queue");
    }

    pub fn tick(&mut self) {
        let current_time_value = time::now().into_raw();
        while current_time_value > NEXT_RESUME_TIME.load(Ordering::Relaxed) {
            let mut sleeping_tasks = SLEEPING_TASKS.lock();
            if let Some(SleepingTask { waker, .. }) = sleeping_tasks.pop() {
                // log::trace!("Waking @ {resume_time:?}...");
                waker.wake();

                match sleeping_tasks.peek() {
                    Some(SleepingTask { resume_time, .. }) => {
                        NEXT_RESUME_TIME.store(resume_time.into_raw(), Ordering::Relaxed);
                    }
                    None => {
                        NEXT_RESUME_TIME.store(u64::MAX, Ordering::Relaxed);
                    }
                }
            }
        }

        while let Some(id) = self.task_queue.pop() {
            let task = match self.tasks.get_mut(&id) {
                Some(task) => task,
                None => continue,
            };
            let waker = self
                .waker_cache
                .entry(id)
                .or_insert_with(|| TaskWaker::new(id, self.task_queue.clone()));
            let mut context = Context::from_waker(waker);
            match task.future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {
                    self.tasks.remove(&id);
                    self.waker_cache.remove(&id);
                }
                Poll::Pending => {}
            }
        }
    }
}

struct TaskWaker {
    task_id: u64,
    task_queue: Arc<ArrayQueue<u64>>,
}

impl TaskWaker {
    fn new(task_id: u64, task_queue: Arc<ArrayQueue<u64>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.task_queue
            .push(self.task_id)
            .expect("too many tasks in queue");
    }
}

pub fn sleep(duration: Duration) -> Sleep {
    let current_time = time::now();
    let resume_time = current_time + duration;

    Sleep(resume_time)
}

pub struct Sleep(Instant);

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let current_time = time::now();

        if let Some(duration) = self.0.checked_duration_since(current_time) {
            let resume_time = current_time + duration;
            let waker = cx.waker().clone();

            SLEEPING_TASKS
                .lock()
                .push(SleepingTask { resume_time, waker });

            let next_resume_time = Instant::from_raw(NEXT_RESUME_TIME.load(Ordering::SeqCst));
            if resume_time < next_resume_time {
                NEXT_RESUME_TIME.store(resume_time.into_raw(), Ordering::SeqCst);
            }

            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

static SLEEPING_TASKS: Mutex<BinaryHeap<SleepingTask>> = Mutex::new(BinaryHeap::new());
static NEXT_RESUME_TIME: AtomicU64 = AtomicU64::new(u64::MAX);

struct SleepingTask {
    resume_time: Instant,
    waker: Waker,
}

impl Eq for SleepingTask {}

impl PartialEq for SleepingTask {
    fn eq(&self, other: &Self) -> bool {
        self.resume_time == other.resume_time
    }
}

impl Ord for SleepingTask {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        other.resume_time.cmp(&self.resume_time)
    }
}

impl PartialOrd for SleepingTask {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
