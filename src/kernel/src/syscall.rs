use alloc::vec::Vec;
use core::fmt;

use num_enum::{FromPrimitive, IntoPrimitive};

use crate::scheduler::{SCHEDULER, Task};

#[derive(Debug, Clone, Copy, Eq, PartialEq, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Syscall {
    AttachIrq = 0,
    CreateTask = 1,
    RemoveTask = 2,
    TaskSleep = 3,
    MapMemory = 10,
    UnmapMemory = 11,
    GrantMemory = 12,
    Send = 20,
    Receive = 21,
    IpcCall = 22,
    #[num_enum(catch_all)]
    Invalid(u8) = 255,
}

impl From<u64> for Syscall {
    fn from(value: u64) -> Self {
        let id_u8 = value as u8;
        let syscall: Syscall = id_u8.into();
        syscall
    }
}

#[derive(Debug)]
pub enum SysError {
    InvalidValue,
    UnknownSyscall(u8),
}

impl fmt::Display for SysError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl core::error::Error for SysError {}

/// Handle inbound syscall.
#[inline]
pub fn handler<I: Into<Syscall>>(
    id: I,
    args: Vec<u64>,
) -> Result<(), SysError> {
    let id = id.into();

    match id {
        Syscall::AttachIrq => unimplemented!(),
        Syscall::CreateTask => {
            if args.len() < 3 {
                return Err(SysError::InvalidValue);
            }

            // Unfinished and unsafe implementation.
            // let f: extern "C" fn() = unsafe { core::mem::transmute(args[0])
            // }; let task = Task::new(args[1], async move {
            // f();
            // });
            // SCHEDULER.get().unwrap().get_mut().spawn(task);
        },
        Syscall::Invalid(id) => return Err(SysError::UnknownSyscall(id)),
        _ => unimplemented!(),
    };

    Ok(())
}
