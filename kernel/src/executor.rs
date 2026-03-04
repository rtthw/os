//! # Asynchronous Executor

use {
    alloc::{boxed::Box, collections::btree_map::BTreeMap, sync::Arc, task::Wake},
    core::{
        pin::Pin,
        sync::atomic::{AtomicU64, Ordering},
        task::{Context, Poll, Waker},
    },
    crossbeam_queue::ArrayQueue,
};



pub struct Task {
    id: u64,
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
                id,
                future: Box::pin(future),
            },
        );
        self.task_queue.push(id).expect("too many tasks in queue");
    }

    pub fn tick(&mut self) {
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
