//! Endpoint objects for synchronous IPC.

use core::ptr::NonNull;

use crate::error::Result;
use crate::objects::tcb::{IpcState, Tcb, TcbQueue, ThreadState};
use crate::objects::{CapRaw, CapRef, CapRights, ObjType};
use crate::scheduler::SCHEDULER;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum EndpointState {
    #[default]
    Idle = 0,
    Send = 1,
    Recv = 2,
}

#[repr(C)]
#[derive(Debug)]
pub struct EndpointObj {
    state: EndpointState,
    queue: TcbQueue,
}

impl EndpointObj {
    /// Create a new idle endpoint.
    pub const fn new() -> Self {
        Self {
            state: EndpointState::Idle,
            queue: TcbQueue::new(),
        }
    }

    pub fn state(&self) -> EndpointState {
        self.state
    }

    pub fn queue(&self) -> &TcbQueue {
        &self.queue
    }
}

impl Default for EndpointObj {
    fn default() -> Self {
        Self::new()
    }
}

pub type EndpointCap<'a> = CapRef<'a, EndpointObj>;

impl EndpointCap<'_> {
    const BADGE_OFFSET: usize = 0;
    const BADGE_WIDTH: usize = 28;

    /// Create a new endpoint capability.
    pub const fn mint(
        paddr: usize,
        badge: usize,
        rights: CapRights,
    ) -> CapRaw {
        let arg1 = badge & ((1 << Self::BADGE_WIDTH) - 1);

        let mut capraw = CapRaw::default_with_type(ObjType::Endpoint);
        capraw.paddr = paddr;
        capraw.arg1 = arg1;
        capraw.rights = rights;
        capraw
    }

    pub fn badge(&self) -> usize {
        let raw = self.raw.get();
        (raw.arg1 >> Self::BADGE_OFFSET) & ((1 << Self::BADGE_WIDTH) - 1)
    }

    #[inline]
    pub fn can_send(&self) -> bool {
        self.rights().contains(CapRights::SEND)
    }

    #[inline]
    pub fn can_receive(&self) -> bool {
        self.rights().contains(CapRights::RECEIVE)
    }

    #[inline]
    pub fn can_grant(&self) -> bool {
        self.rights().contains(CapRights::GRANT)
    }

    /// Get a mutable reference to the endpoint object.
    ///
    /// # Safety
    /// Caller must ensure exclusive access.
    unsafe fn as_object_mut(&self) -> &'static mut EndpointObj {
        &mut *(self.paddr().as_u64() as *mut EndpointObj)
    }

    unsafe fn as_object(&self) -> &'static EndpointObj {
        &*(self.paddr().as_u64() as *const EndpointObj)
    }

    pub fn identify(&self, tcb: &mut Tcb) -> usize {
        tcb.set_mr(Tcb::MR1, self.cap_type() as usize);
        tcb.set_mr(Tcb::MR2, self.paddr().as_u64() as usize);
        tcb.set_mr(Tcb::MR3, self.badge());
        tcb.set_mr(Tcb::MR4, self.rights().bits() as usize);
        4
    }
}

/// Perform IPC transfer from sender to receiver.
///
/// # Safety
/// Both TCB pointers must be valid.
unsafe fn do_ipc_transfer(
    sender: NonNull<Tcb>,
    receiver: NonNull<Tcb>,
    badge: usize,
    can_grant: bool,
) {
    let sender_ref = sender.as_ref();
    let receiver_ptr = receiver.as_ptr();

    (*receiver_ptr).set_mr(Tcb::MR1, badge);

    // Transfer message registers.
    for i in 0..4 {
        let val = sender_ref.get_mr(Tcb::MR1 + i);
        (*receiver_ptr).set_mr(Tcb::MR1 + i, val);
    }

    // TODO: Handle capability transfer if can_grant is true.
    let _ = can_grant;
}

/// Handle failed non-blocking receive.
unsafe fn do_nb_recv_failed_transfer(thread: NonNull<Tcb>) {
    let thread_ptr = thread.as_ptr();
    // Set badge to 0 to indicate no message.
    (*thread_ptr).set_mr(Tcb::MR1, 0);
}

/// Send IPC message.
///
/// # Safety
/// Caller must ensure all pointers are valid.
pub unsafe fn send_ipc(
    blocking: bool,
    do_call: bool,
    badge: usize,
    can_grant: bool,
    can_grant_reply: bool,
    sender: NonNull<Tcb>,
    endpoint: &EndpointCap<'_>,
) -> Result<()> {
    let ep = endpoint.as_object_mut();
    let sender_ptr = sender.as_ptr();

    match ep.state {
        EndpointState::Idle | EndpointState::Send => {
            if blocking {
                // Store IPC state in sender's TCB.
                (*sender_ptr).ipc_state = IpcState {
                    badge,
                    can_grant,
                    can_grant_reply,
                    is_call: do_call,
                };

                // Block sender.
                (*sender_ptr).state = ThreadState::BlockedOnSend;
                (*sender_ptr).blocking_object = Some(NonNull::new_unchecked(
                    ep as *mut EndpointObj as *mut u8,
                ));

                ep.queue.append(sender);
                ep.state = EndpointState::Send;

                // Scheduler will handle context switch.
            }
            Ok(())
        },
        EndpointState::Recv => {
            let receiver = ep
                .queue
                .dequeue_head()
                .expect("Receive endpoint queue must not be empty");

            // Update endpoint state.
            if ep.queue.is_empty() {
                ep.state = EndpointState::Idle;
            }

            do_ipc_transfer(sender, receiver, badge, can_grant);

            let receiver_ptr = receiver.as_ptr();

            if do_call {
                if can_grant || can_grant_reply {
                    setup_caller_cap(sender, receiver)?;
                } else {
                    // No reply possible, make sender inactive.
                    (*sender_ptr).state = ThreadState::Inactive;
                }
            }

            (*receiver_ptr).state = ThreadState::Running;

            // Schedule receiver (preempt current thread).
            if let Some(sched) = SCHEDULER.get() {
                (*receiver_ptr).state = ThreadState::Inactive;
                let _ = sched.get_mut().wake(receiver);
            }

            Ok(())
        },
    }
}

/// Receive IPC message.
///
/// # Safety
/// Caller must ensure all pointers are valid.
pub unsafe fn receive_ipc(
    receiver: NonNull<Tcb>,
    endpoint: &EndpointCap<'_>,
    is_blocking: bool,
) -> Result<()> {
    let ep = endpoint.as_object_mut();
    let receiver_ptr = receiver.as_ptr();

    // TODO: Check for bound notification first (like seL4 does).

    match ep.state {
        EndpointState::Idle | EndpointState::Recv => {
            if is_blocking {
                // Block receiver.
                (*receiver_ptr).state = ThreadState::BlockedOnReceive;
                (*receiver_ptr).blocking_object = Some(
                    NonNull::new_unchecked(ep as *mut EndpointObj as *mut u8),
                );

                // Add to endpoint queue.
                ep.queue.append(receiver);
                ep.state = EndpointState::Recv;

                // Scheduler will handle context switch.
            } else {
                // Non-blocking receive with no sender.
                do_nb_recv_failed_transfer(receiver);
            }
            Ok(())
        },

        EndpointState::Send => {
            // Dequeue first sender.
            let sender = ep
                .queue
                .dequeue_head()
                .expect("Send endpoint queue must not be empty");

            // Update endpoint state.
            if ep.queue.is_empty() {
                ep.state = EndpointState::Idle;
            }

            let sender_ptr = sender.as_ptr();
            let ipc_state = (*sender_ptr).ipc_state;

            do_ipc_transfer(
                sender,
                receiver,
                ipc_state.badge,
                ipc_state.can_grant,
            );

            // Handle call semantics.
            if ipc_state.is_call {
                if ipc_state.can_grant || ipc_state.can_grant_reply {
                    setup_caller_cap(sender, receiver)?;
                } else {
                    (*sender_ptr).state = ThreadState::Inactive;
                }
            } else {
                (*sender_ptr).state = ThreadState::Inactive;
                if let Some(sched) = SCHEDULER.get() {
                    let _ = sched.get_mut().wake(sender);
                }
            }

            Ok(())
        },
    }
}

/// Setup reply capability for call operations.
///
/// # Safety
/// Both TCB pointers must be valid.
unsafe fn setup_caller_cap(
    caller: NonNull<Tcb>,
    callee: NonNull<Tcb>,
) -> Result<()> {
    let caller_ptr = caller.as_ptr();

    // Block caller waiting for reply.
    (*caller_ptr).state = ThreadState::BlockedOnReply;
    (*caller_ptr).reply_to = Some(callee);

    // Store reply cap in callee's TCB.
    let callee_ptr = callee.as_ptr();
    (*callee_ptr).caller = Some(caller);

    Ok(())
}

/// Reply from kernel with error.
pub fn reply_from_kernel_error(thread: &mut Tcb, error_type: usize) {
    thread.set_mr(Tcb::MR1, 0);
    thread.set_mr(Tcb::MR2, error_type);
}

/// Reply from kernel with success (empty message).
pub fn reply_from_kernel_success_empty(thread: &mut Tcb) {
    thread.set_mr(Tcb::MR1, 0);
    // Message info with 0 length, 0 caps, 0 extra caps, label = 0.
}

/// Cancel IPC for a thread.
///
/// This removes a thread from any endpoint queue it may be in.
///
/// # Safety
/// TCB pointer must be valid.
pub unsafe fn cancel_ipc(tcb: NonNull<Tcb>) {
    let tcb_ptr = tcb.as_ptr();
    let state = (*tcb_ptr).state;

    match state {
        ThreadState::BlockedOnSend | ThreadState::BlockedOnReceive => {
            // Get the endpoint this thread is blocked on.
            if let Some(ep_ptr) = (*tcb_ptr).blocking_object {
                let ep = &mut *(ep_ptr.as_ptr() as *mut EndpointObj);
                ep.queue.remove(tcb);

                if ep.queue.is_empty() {
                    ep.state = EndpointState::Idle;
                }
            }

            (*tcb_ptr).state = ThreadState::Inactive;
            (*tcb_ptr).blocking_object = None;
        },

        ThreadState::BlockedOnNotification => {
            // TODO: Cancel notification signal.
            (*tcb_ptr).state = ThreadState::Inactive;
        },

        ThreadState::BlockedOnReply => {
            // Clear reply relationship.
            if let Some(callee) = (*tcb_ptr).reply_to {
                (*callee.as_ptr()).caller = None;
            }
            (*tcb_ptr).reply_to = None;
            (*tcb_ptr).state = ThreadState::Inactive;
        },

        _ => {},
    }
}

/// Cancel all IPC on an endpoint.
///
/// This is called when an endpoint capability is revoked.
///
/// # Safety
/// Endpoint pointer must be valid.
pub unsafe fn cancel_all_ipc(endpoint: &EndpointCap<'_>) {
    let ep = endpoint.as_object_mut();

    if ep.state == EndpointState::Idle {
        return;
    }

    // Restart all blocked threads.
    while let Some(tcb) = ep.queue.dequeue_head() {
        let tcb_ptr = tcb.as_ptr();

        (*tcb_ptr).state = ThreadState::Restart;
        (*tcb_ptr).blocking_object = None;

        // Re-enqueue in scheduler.
        if let Some(sched) = SCHEDULER.get() {
            (*tcb_ptr).state = ThreadState::Inactive;
            let _ = sched.get_mut().enqueue(tcb);
        }
    }

    ep.state = EndpointState::Idle;
}

/// Cancel sends with a specific badge.
///
/// # Safety
/// Endpoint pointer must be valid.
pub unsafe fn cancel_badged_sends(endpoint: &EndpointCap<'_>, badge: usize) {
    let ep = endpoint.as_object_mut();

    if ep.state != EndpointState::Send {
        return;
    }

    // Build a new queue with non-matching threads.
    let mut current = ep.queue.head;
    let mut new_queue = TcbQueue::new();

    while let Some(tcb) = current {
        let tcb_ptr = tcb.as_ptr();
        let next = (*tcb_ptr).ep_next;

        if (*tcb_ptr).ipc_state.badge == badge {
            // Restart this thread.
            (*tcb_ptr).ep_next = None;
            (*tcb_ptr).ep_prev = None;
            (*tcb_ptr).state = ThreadState::Restart;
            (*tcb_ptr).blocking_object = None;

            // Re-enqueue in scheduler.
            if let Some(sched) = SCHEDULER.get() {
                (*tcb_ptr).state = ThreadState::Inactive;
                let _ = sched.get_mut().enqueue(tcb);
            }
        } else {
            // Keep in queue.
            new_queue.append(tcb);
        }

        current = next;
    }

    ep.queue = new_queue;

    if ep.queue.is_empty() {
        ep.state = EndpointState::Idle;
    }
}
