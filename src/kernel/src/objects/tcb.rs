//! Thread control blocks.

use core::ptr::NonNull;

use crate::arch::trapframe::TrapFrame;
use crate::cspace::CSpace;
use crate::error::Result;
use crate::objects::capability::{CapRaw, CapRef, ObjType};
use crate::objects::cnode::CNodeEntry;

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
    // RDI.
    pub const MR1: usize = 5;
    // RSI.
    pub const MR2: usize = 4;
    // RDX.
    pub const MR3: usize = 3;
    // RCX.
    pub const MR4: usize = 2;
    // R8.
    pub const MR5: usize = 7;
    // R9.
    pub const MR6: usize = 8;
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
