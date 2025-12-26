/// Custom system error.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum SysError {
    CSpaceNotFound = 1,
    CapabilityTypeError,
    LookupError,
    UnableToDerive,
    SlotNotEmpty,
    SlotEmpty,
    UnsupportedSyscallOp,
    VSpaceCapMapped,
    VSpaceCapNotMapped,
    VSpaceTableMiss,
    VSpaceSlotOccupied,
    VSpacePermissionError,
    InvalidValue,
    SizeTooSmall,
}

pub type Result<T> = core::result::Result<T, SysError>;
