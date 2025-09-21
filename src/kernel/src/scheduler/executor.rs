extern crate alloc;

use alloc::collections::{BTreeMap, BinaryHeap};
use alloc::sync::Arc;
use alloc::task::Wake;
use core::cell::UnsafeCell;
use core::cmp::Reverse;
use core::task::{Context, Poll, Waker};

use super::{Task, TaskId};
use crate::arch;

type Queue = UnsafeCell<BinaryHeap<Reverse<DeadlineEntry>>>;

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

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Sync for Executor {}

impl Executor {
    /// Create a new [`Executor`].
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: UnsafeCell::new(BinaryHeap::with_capacity(
                DEFAULT_CAPACITY,
            )),
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

        let queue = unsafe { &mut *self.task_queue.get() };
        queue.push(Reverse(DeadlineEntry { deadline, task_id }))
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
                let queue = unsafe { &mut *task_queue.get() };
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

            let waker = waker_cache.entry(task_id).or_insert_with(|| {
                TaskWaker::new(task_id, task.deadline, task_queue as *mut _)
            });

            let mut context = Context::from_waker(waker);
            match task.future.as_mut().poll(&mut context) {
                Poll::Ready(()) => {
                    tasks.remove(&task_id);
                    waker_cache.remove(&task_id);
                },
                Poll::Pending => {},
            }
        }
    }

    fn sleep_if_idle(&self) {
        arch::halt(unsafe { (*self.task_queue.get()).is_empty() });
    }
}

struct TaskWaker {
    task_id: TaskId,
    deadline: u64,
    task_queue: *mut Queue,
}

// Since scheduler is per-core, it shouldn't be transfered to another thread.
unsafe impl Sync for TaskWaker {}
unsafe impl Send for TaskWaker {}

impl TaskWaker {
    #[allow(clippy::new_ret_no_self)]
    fn new(task_id: TaskId, deadline: u64, task_queue: *mut Queue) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            deadline,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        let queue = unsafe { (*self.task_queue).get_mut() };
        queue.push(Reverse(DeadlineEntry {
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
