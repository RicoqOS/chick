use core::ptr::NonNull;

use crate::arch::trapframe::TrapFrame;
use crate::objects::capacity::{CNodeEntry, CapRef};

#[derive(Debug)]
pub enum FaultInfo {
    PageFault { addr: usize },
    SyscallFault { syscall: usize },
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub enum ThreadState {
    #[default]
    Inactive,
    Running,
    Restart,
    BlockedOnReceive,
    BlockedOnSend,
    BlockedOnReply,
    BlockedOnNotification,
    RunningVm,
    IdleThreadState,
}

impl ThreadState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreadState::Inactive => "inactive",
            ThreadState::Running => "running",
            ThreadState::Restart => "restart",
            ThreadState::BlockedOnReceive => "blocked on recv",
            ThreadState::BlockedOnSend => "blocked on send",
            ThreadState::BlockedOnReply => "blocked on reply",
            ThreadState::BlockedOnNotification => "blocked on ntfn",
            ThreadState::RunningVm => "running VM",
            ThreadState::IdleThreadState => "idle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    CapFault {
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
#[repr(align(2048))]
#[derive(Debug)]
pub struct Tcb {
    /// Arch specific tcb state (including context).
    pub context: TrapFrame,

    /// Notification that this TCB is bound to. If this is set, when this TCB waits on
    /// any sync endpoint, it may receive a signal from a Notification object.
    notification: usize,

    /// Current fault.
    fault: Option<Fault>,
    fault_ep: CNodeEntry,

    /// Scheduling context that this tcb is running on, if it is NULL the tcb cannot
    /// be in the scheduler queues.
    pub sched_context: Option<NonNull<SchedContext>>,

    /// Timeslice remaining.
    time_slice: usize,

    /// Priority.
    pub priority: u8,

    /// Userland virtual address of thread IPC buffer.
    pub ipc_buffer: CNodeEntry,

    /// Capability-based root space.
    pub cspace_root: CNodeEntry,
    vspace_root: CNodeEntry,

    /// Thread state.
    pub state: ThreadState,
}

#[derive(Debug)]
#[repr(C)]
struct SchedContext {
    /// Controls rate at which budget is replenished.
    ticks: usize,

    /// Amount of ticks scheduled for since seL4_SchedContext_Consumed
    /// was last called or a timeout exception fired.
    ticks_consumed: usize,

    /// Thread that this scheduling context is bound to.
    thread: u8,
}

pub const TCB_SIZE: usize = size_of::<Tcb>().next_power_of_two();

pub type TcbCap<'a> = CapRef<'a, Tcb>;

impl Tcb {
    pub const fn new() -> Self {
        Self {
            context: TrapFrame::new(),
            notification: 0,
            sched_context: None,
            time_slice: 0,
            priority: 10,
            ipc_buffer: CNodeEntry::new(),
            cspace_root: CNodeEntry::new(),
            vspace_root: CNodeEntry::new(),
            fault: None,
            fault_ep: CNodeEntry::new(),
            state: ThreadState::Running,
        }
    }
}
