//! Deterministic, allocation-free EDF scheduler (per-core)

use core::ptr::NonNull;

use heapless::BinaryHeap;
use heapless::binary_heap::Min;

use crate::arch;
use crate::objects::tcb::{Tcb, ThreadState};

/// Maximum amount of TCB entry on a scheduler.
const MAX_TCB_PER_CORE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Entry {
    deadline: u64,
    tcb: NonNull<Tcb>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedError {
    QueueFull,
    InvalidState,
    NotFound,
}

/// Per-core EDF executor
#[derive(Default)]
pub struct Executor {
    ready: BinaryHeap<Entry, Min, MAX_TCB_PER_CORE>,
    current: Option<Entry>,
    idle: Option<NonNull<Tcb>>,
}

// SAFETY: executor is per-core, interrupts disabled during mutation
unsafe impl Send for Executor {}

impl Executor {
    /// Create new [`Executor`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ready: BinaryHeap::new(),
            current: None,
            idle: None,
        }
    }

    pub unsafe fn set_idle(&mut self, tcb: NonNull<Tcb>) {
        (*tcb.as_ptr()).state = ThreadState::Ready;
        self.idle = Some(tcb);
    }

    #[inline]
    fn deadline_of(tcb: NonNull<Tcb>) -> u64 {
        unsafe {
            tcb.as_ref()
                .sched_context
                .map(|sc| sc.as_ref().deadline)
                .unwrap_or(u64::MAX)
        }
    }

    /// Insert a [`ThreadState::Ready`] [`Tcb`] in queue.
    pub unsafe fn enqueue(
        &mut self,
        tcb: NonNull<Tcb>,
    ) -> Result<(), SchedError> {
        let tcb_ref = tcb.as_ref();

        if tcb_ref.state != ThreadState::Ready {
            return Err(SchedError::InvalidState);
        }

        let entry = Entry {
            deadline: Self::deadline_of(tcb),
            tcb,
        };

        self.ready.push(entry).map_err(|_| SchedError::QueueFull)
    }

    #[inline]
    fn should_preempt(&self) -> bool {
        match (self.current, self.ready.peek()) {
            (Some(cur), Some(next)) => next.deadline < cur.deadline,
            (None, Some(_)) => true,
            _ => false,
        }
    }

    unsafe fn context_switch(&mut self, next: Entry) -> ! {
        if let Some(cur) = self.current.take() {
            let cur_tcb = cur.tcb.as_ptr();
            if (*cur_tcb).state == ThreadState::Running {
                (*cur_tcb).state = ThreadState::Ready;
                self.ready.push(cur).expect("ready queue overflow");
            }
        }

        let tcb = next.tcb.as_ptr();
        (*tcb).state = ThreadState::Running;
        self.current = Some(next);

        (*tcb).context.restore()
    }

    /// Called from timer interrupt
    pub unsafe fn preempt(&mut self) {
        if !self.should_preempt() {
            return;
        }

        let next = self.ready.pop().expect("preempt without ready task");
        self.context_switch(next);
    }

    pub unsafe fn yield_current(&mut self) -> ! {
        if let Some(cur) = self.current.take() {
            let tcb = cur.tcb.as_ptr();
            (*tcb).state = ThreadState::Ready;
            self.ready.push(cur).expect("ready queue overflow");
        }

        self.schedule()
    }

    pub unsafe fn block_current(&mut self, state: ThreadState) -> ! {
        let valid = matches!(
            state,
            ThreadState::BlockedOnReceive |
                ThreadState::BlockedOnSend |
                ThreadState::BlockedOnReply |
                ThreadState::BlockedOnNotification
        );

        assert!(valid);

        if let Some(cur) = self.current.take() {
            (*cur.tcb.as_ptr()).state = state;
        }

        self.schedule()
    }

    pub unsafe fn wake(
        &mut self,
        tcb: NonNull<Tcb>,
    ) -> Result<(), SchedError> {
        let tcb_ref = tcb.as_ref();

        if !matches!(
            tcb_ref.state,
            ThreadState::BlockedOnReceive |
                ThreadState::BlockedOnSend |
                ThreadState::BlockedOnReply |
                ThreadState::BlockedOnNotification
        ) {
            return Err(SchedError::InvalidState);
        }

        (*tcb.as_ptr()).state = ThreadState::Ready;
        self.enqueue(tcb)
    }

    unsafe fn schedule(&mut self) -> ! {
        if let Some(next) = self.ready.pop() {
            self.context_switch(next);
        }

        if let Some(idle) = self.idle {
            let entry = Entry {
                tcb: idle,
                deadline: u64::MAX,
            };
            self.context_switch(entry);
        }

        loop {
            arch::halt();
        }
    }

    pub fn run(&mut self) -> ! {
        unsafe { self.schedule() }
    }
}
