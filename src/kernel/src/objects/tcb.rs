//! Thread control blocks.

use core::ptr::NonNull;

use crate::arch::trapframe::TrapFrame;
use crate::cspace::CSpace;
use crate::error::Result;
use crate::objects::cnode::CNodeEntry;
use crate::objects::{CapRaw, CapRef, ObjType};

// Forward declaration for Endpoint to avoid circular dependency.
pub struct EndpointPtr(pub *mut u8);

/// IPC transfer details stored in blocked thread state.
#[derive(Debug, Clone, Copy, Default)]
pub struct IpcState {
    /// Badge to transfer.
    pub badge: usize,
    /// Can grant capabilities.
    pub can_grant: bool,
    /// Can grant reply capability.
    pub can_grant_reply: bool,
    /// Is this a call (expects reply).
    pub is_call: bool,
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum ThreadState {
    #[default]
    Inactive,
    Ready,
    Running,
    Restart,
    BlockedOnReceive,
    BlockedOnSend,
    BlockedOnReply,
    BlockedOnNotification,
    RunningVm,
    Idle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    Cap {
        address: usize,
        in_receive_phase: bool,
    },
    UnknownSyscall {
        syscall_number: usize,
    },
    UserException {
        number: usize,
        code: usize,
    },
    Timeout {
        badge: usize,
    },
    DebugException {
        exception_reason: usize,
        breakpoint_address: usize,
        breakpoint_number: usize,
    },
    Unknown {
        fault_type_raw: usize,
    },
}

/// Thread control block as defined on seL4 kernel.
#[repr(C)]
#[repr(align(1024))]
#[derive(Debug)]
pub struct Tcb {
    /// Arch specific tcb state (including context).
    pub context: TrapFrame,

    /// Notification that this TCB is bound to. If this is set, when this TCB
    /// waits on any sync endpoint, it may receive a signal from a
    /// Notification object.
    notification: usize,

    /// Current fault.
    fault: Option<Fault>,
    fault_ep: CNodeEntry,

    /// Scheduling context that this tcb is running on, if it is NULL the tcb
    /// cannot be in the scheduler queues.
    pub sched_context: Option<NonNull<SchedContext>>,

    /// Userland virtual address of thread IPC buffer.
    pub ipc_buffer: CNodeEntry,

    /// Capability-based root space.
    pub cspace_root: CNodeEntry,
    vspace_root: CNodeEntry,

    /// Thread state.
    pub state: ThreadState,

    /// Next TCB in endpoint queue.
    pub ep_next: Option<NonNull<Tcb>>,
    /// Previous TCB in endpoint queue.
    pub ep_prev: Option<NonNull<Tcb>>,

    /// Object this thread is blocked on (endpoint pointer as raw).
    pub blocking_object: Option<NonNull<u8>>,
    pub ipc_state: IpcState,
    /// When `BlockedOnReply` TCB we're waiting for reply from.
    pub reply_to: Option<NonNull<Tcb>>,
    /// TCB calling and waiting for reply.
    pub caller: Option<NonNull<Tcb>>,
}

#[derive(Debug)]
#[repr(C)]
pub struct SchedContext {
    /// Controls rate at which budget is replenished.
    ticks: usize,

    /// Amount of ticks scheduled for since seL4_SchedContext_Consumed
    /// was last called or a timeout exception fired.
    ticks_consumed: usize,

    /// Deadline for RTOS.
    pub deadline: u64,

    /// Thread that this scheduling context is bound to.
    thread: u8,
}

impl Tcb {
    /// Create a new [`Tcb`].
    pub const fn new() -> Self {
        Self {
            context: TrapFrame::new(),
            notification: 0,
            sched_context: None,
            ipc_buffer: CNodeEntry::new(),
            cspace_root: CNodeEntry::new(),
            vspace_root: CNodeEntry::new(),
            fault: None,
            fault_ep: CNodeEntry::new(),
            state: ThreadState::Running,
            ep_next: None,
            ep_prev: None,
            blocking_object: None,
            ipc_state: IpcState {
                badge: 0,
                can_grant: false,
                can_grant_reply: false,
                is_call: false,
            },
            reply_to: None,
            caller: None,
        }
    }

    /// Extract [`CSpace`] root of current [`Tcb`].
    pub fn cspace(&self) -> Result<CSpace<'_>> {
        CSpace::new(&self.cspace_root)
    }

    pub fn get_mr(&self, idx: usize) -> usize {
        self.context.get_mr(idx)
    }

    pub fn set_mr(&mut self, idx: usize, mr: usize) {
        self.context.set_mr(idx, mr)
    }
}

#[cfg(target_arch = "x86_64")]
impl Tcb {
    // RDI. Capacity badge.
    pub const MR1: usize = 0;
    // RSI. Message info tag.
    pub const MR2: usize = 1;
    // R10.
    pub const MR3: usize = 9;
    // R8.
    pub const MR4: usize = 10;
    // R9.
    pub const MR5: usize = 11;
    // R15.
    pub const MR6: usize = 12;
}

pub type TcbCap<'a> = CapRef<'a, Tcb>;

impl TcbCap<'_> {
    pub const fn mint(paddr: usize) -> CapRaw {
        let mut capraw = CapRaw::default_with_type(ObjType::Tcb);
        capraw.paddr = paddr;
        capraw
    }

    pub fn identify(&self, tcb: &mut Tcb) -> usize {
        tcb.set_mr(Tcb::MR1, self.cap_type() as usize);
        1
    }
}

/// Queue of TCBs waiting on an endpoint.
#[derive(Debug, Default, Clone, Copy)]
pub struct TcbQueue {
    pub head: Option<NonNull<Tcb>>,
    pub tail: Option<NonNull<Tcb>>,
}

impl TcbQueue {
    /// Create an empty [`TcbQueue`].
    pub const fn new() -> Self {
        Self {
            head: None,
            tail: None,
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// Append a TCB to the queue.
    ///
    /// # Safety
    /// Caller must ensure `tcb` is valid and not already in a queue.
    pub unsafe fn append(&mut self, tcb: NonNull<Tcb>) {
        let tcb_ptr = tcb.as_ptr();

        // Clear TCB's queue pointers.
        (*tcb_ptr).ep_next = None;
        (*tcb_ptr).ep_prev = self.tail;

        match self.tail {
            Some(tail) => {
                (*tail.as_ptr()).ep_next = Some(tcb);
            },
            None => {
                self.head = Some(tcb);
            },
        }

        self.tail = Some(tcb);
    }

    /// Remove and return the head of the queue.
    ///
    /// # Safety
    /// Caller must ensure queue TCBs are valid.
    pub unsafe fn dequeue_head(&mut self) -> Option<NonNull<Tcb>> {
        let head = self.head?;
        let head_ptr = head.as_ptr();

        self.head = (*head_ptr).ep_next;

        match self.head {
            Some(new_head) => {
                (*new_head.as_ptr()).ep_prev = None;
            },
            None => {
                self.tail = None;
            },
        }

        (*head_ptr).ep_next = None;
        (*head_ptr).ep_prev = None;

        Some(head)
    }

    /// Remove a specific TCB from the queue.
    ///
    /// # Safety
    /// Caller must ensure `tcb` is in this queue.
    pub unsafe fn remove(&mut self, tcb: NonNull<Tcb>) {
        let tcb_ptr = tcb.as_ptr();
        let prev: Option<NonNull<Tcb>> = (*tcb_ptr).ep_prev;
        let next: Option<NonNull<Tcb>> = (*tcb_ptr).ep_next;

        match prev {
            Some(prev_ptr) => {
                (*prev_ptr.as_ptr()).ep_next = next;
            },
            None => {
                self.head = next;
            },
        }

        match next {
            Some(next_ptr) => {
                (*next_ptr.as_ptr()).ep_prev = prev;
            },
            None => {
                self.tail = prev;
            },
        }

        (*tcb_ptr).ep_next = None;
        (*tcb_ptr).ep_prev = None;
    }
}
