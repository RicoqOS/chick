//! A simple notion of a task, which is a future that can be polled by an
//! executor.
extern crate alloc;

use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, Ordering};
use core::task::{Context, Poll};

use crate::objects::tcb::Tcb;

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// A unique identifier for a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(pub u64);

impl TaskId {
    /// Create a new, unique task ID.
    fn new() -> Self {
        TaskId(NEXT_ID.fetch_add(1, Ordering::AcqRel))
    }
}

/// A task executed by an executor.
pub struct Task {
    pub id: TaskId,
    pub tcb: Option<NonNull<Tcb>>,
    pub future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    /// Create a new task from a future.
    pub fn new(
        tcb: Option<NonNull<Tcb>>,
        future: impl Future<Output = ()> + 'static,
    ) -> Task {
        Task {
            id: TaskId::new(),
            tcb,
            future: Box::pin(future),
        }
    }

    /// Poll the task.
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}
