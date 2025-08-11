extern crate alloc;

use spin::RwLock;

use alloc::collections::{BTreeMap, BinaryHeap};
use alloc::sync::Arc;
use alloc::task::Wake;
use core::cmp::Reverse;
use core::task::{Context, Poll, Waker};

use super::{Task, TaskId};

type Queue = Arc<RwLock<BinaryHeap<Reverse<DeadlineEntry>>>>;

/// Default amount of tasks in queue.
const DEFAULT_CAPACITY: usize = 100;

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
struct DeadlineEntry {
    deadline: u64,
    task_id: TaskId,
}

/// Task executor that drives tasks to completion.
pub struct Executor {
    /// Binary tree of running tasks.
    tasks: BTreeMap<TaskId, Task>,
    /// Priority queue of tasks (min-heap on deadline).
    task_queue: Queue,
    /// Cache of wakers for tasks.
    waker_cache: BTreeMap<TaskId, Waker>,
}

impl Executor {
    /// Create a new [`Executor`].
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: Arc::new(RwLock::new(BinaryHeap::with_capacity(DEFAULT_CAPACITY))),
            waker_cache: BTreeMap::new(),
        }
    }

    /// Spawn a new task.
    ///
    /// # Safety
    /// * panics if the task ID is already in the executor
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        let deadline = task.deadline;

        if self.tasks.insert(task_id, task).is_some() {
            panic!("task with same ID already in tasks");
        }

        self.task_queue
            .write()
            .push(Reverse(DeadlineEntry { deadline, task_id }));
    }

    /// Run the executor.
    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    /// Run all tasks that are ready to run.
    fn run_ready_tasks(&mut self) {
        let Self {
            tasks,
            task_queue,
            waker_cache,
        } = self;

        loop {
            let next_entry = {
                let mut queue = task_queue.write();
                queue.pop()
            };

            let task_id = match next_entry {
                Some(Reverse(entry)) => entry.task_id,
                None => break,
            };

            let task = match tasks.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };

            let waker = waker_cache
                .entry(task_id)
                .or_insert_with(|| TaskWaker::new(task_id, task.deadline, task_queue.clone()));

            let mut context = Context::from_waker(waker);
            match task.future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    fn sleep_if_idle(&self) {
        use x86_64::instructions::interrupts::{self, enable_and_hlt};

        interrupts::disable();
        if self.task_queue.read().is_empty() {
            enable_and_hlt();
        } else {
            interrupts::enable();
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    deadline: u64,
    task_queue: Queue,
}

impl TaskWaker {
    fn new(task_id: TaskId, deadline: u64, task_queue: Queue) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            deadline,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.write().push(Reverse(DeadlineEntry {
            deadline: self.deadline,
            task_id: self.task_id,
        }));
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
