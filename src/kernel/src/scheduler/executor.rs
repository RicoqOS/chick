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
const DEFAULT_CAPACITY: usize = 20;

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
struct DeadlineEntry {
    deadline: u64,
    task_id: TaskId,
}

/// Task executor that drives tasks to completion.
pub struct Executor {
    tasks: BTreeMap<TaskId, TaskSlot>,
    task_queue: Queue,
    current_task: Option<DeadlineEntry>,
}

struct TaskSlot {
    task: Task,
    waker: Option<Waker>,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Sync for Executor {}

impl Executor {
    /// Create new [`Executor`].
    #[must_use]
    pub fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: UnsafeCell::new(BinaryHeap::with_capacity(
                DEFAULT_CAPACITY,
            )),
            current_task: None,
        }
    }

    /// Spawn a new task.
    ///
    /// # Safety
    /// * panics if the task ID is already in the executor.
    pub fn spawn(&mut self, task: Task) {
        let task_id = task.id;
        let deadline = task.deadline;

        if self.tasks.contains_key(&task_id) {
            panic!("task with same ID already in tasks");
        }

        self.tasks.insert(task_id, TaskSlot { task, waker: None });

        let queue = unsafe { &mut *self.task_queue.get() };
        queue.push(Reverse(DeadlineEntry { deadline, task_id }));
    }

    fn handle_waker_task(&mut self, entry: &DeadlineEntry) {
        let slot = match self.tasks.get_mut(&entry.task_id) {
            Some(s) => s,
            None => return,
        };

        let waker = slot.waker.get_or_insert_with(|| {
            TaskWaker::new(
                entry.task_id,
                entry.deadline,
                &mut self.task_queue as *mut _,
            )
        });

        let mut context = Context::from_waker(waker);
        match slot.task.future.as_mut().poll(&mut context) {
            Poll::Ready(()) => {
                log::debug!("task {} finished", entry.task_id.0);
                self.tasks.remove(&entry.task_id);
                self.current_task = None;
            },
            Poll::Pending => unimplemented!(),
        }
    }

    /// Preempt current task if another with higher priority exists.
    pub fn preempt(&mut self) {
        let queue = unsafe { &mut *self.task_queue.get() };
        let next_entry = queue.peek().copied();

        let Some(current_task) = self.current_task.as_ref() else {
            self.run_ready_tasks();
            return;
        };

        let Some(Reverse(entry)) = next_entry else {
            return;
        };

        if entry.deadline < current_task.deadline {
            log::info!(
                "preempting task #{} (deadline {}) for task #{} (deadline {})",
                current_task.task_id.0,
                current_task.deadline,
                entry.task_id.0,
                entry.deadline
            );

            let _ = queue.pop();

            // Weirdly freeze if task id is 0.
            let task_id = match current_task.task_id.0 {
                0 => crate::scheduler::TaskId(1),
                _ => current_task.task_id,
            };
            queue.push(Reverse(DeadlineEntry {
                deadline: current_task.deadline,
                task_id,
            }));

            self.current_task = Some(entry);
            self.handle_waker_task(&entry);
        }
    }

    /// Run all tasks ready to run.
    fn run_ready_tasks(&mut self) {
        loop {
            let next_entry = {
                let queue = unsafe { &mut *self.task_queue.get() };
                queue.pop()
            };

            let queued_task = match next_entry {
                Some(Reverse(entry)) => entry,
                None => break,
            };

            self.current_task = Some(queued_task);
            self.handle_waker_task(&queued_task);
        }
    }

    fn sleep_if_idle(&self) {
        arch::halt(unsafe { (*self.task_queue.get()).is_empty() });
    }

    /// Like `run_ready_tasks` but sleeps if no task is ready.
    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }
}

struct TaskWaker {
    task_id: TaskId,
    deadline: u64,
    task_queue: *mut Queue,
}

unsafe impl Sync for TaskWaker {}
unsafe impl Send for TaskWaker {}

impl TaskWaker {
    #[must_use]
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
