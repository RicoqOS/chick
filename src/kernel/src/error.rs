//! Custom kernel errors.

pub type Result<T> = core::result::Result<T, SysError>;

/// Custom system error.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(C)]
pub enum SysError {
    None = 0,
    CSpaceNotFound,
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
    OutOfMemory,
    FrameAlreadyMapped,
    FrameNotMapped,
    InvalidOperation,
    AlignmentError,
    RangeError,
    RevokeFailed,
    DeleteFailed,
}

impl SysError {
    /// Convert error to numeric code for syscall return.
    #[inline]
    pub const fn as_code(self) -> usize {
        self as usize
    }

    /// Check if this represents success.
    #[inline]
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::None)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WalkResult {
    /// Found a mapped page at the given level.
    MappedPage {
        paddr: usize,
        size: crate::objects::frame::FrameSize,
        level: usize,
    },
    /// Found an unmapped (not present) entry.
    NotMapped { level: usize },
    /// Found a table entry pointing to next level.
    Table { paddr: usize, level: usize },
}

/// Error during page table operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VSpaceError {
    /// Virtual address is not canonical.
    InvalidVAddr,
    /// Physical address is not properly aligned.
    MisalignedPAddr,
    /// Virtual address is not aligned to page size.
    MisalignedVAddr,
    /// Entry already exists at this location.
    AlreadyMapped,
    /// No mapping exists at this location.
    NotMapped,
    /// Missing intermediate page table.
    MissingTable,
    /// Frame size not supported at this level.
    UnsupportedPageSize,
    /// Invalid ASID.
    InvalidAsid,
    /// Failed to allocate page table.
    AllocationFailed,
}

impl From<VSpaceError> for SysError {
    fn from(e: VSpaceError) -> Self {
        match e {
            VSpaceError::InvalidVAddr => SysError::InvalidValue,
            VSpaceError::MisalignedPAddr => SysError::InvalidValue,
            VSpaceError::MisalignedVAddr => SysError::InvalidValue,
            VSpaceError::AlreadyMapped => SysError::FrameAlreadyMapped,
            VSpaceError::NotMapped => SysError::LookupError,
            VSpaceError::MissingTable => SysError::LookupError,
            VSpaceError::UnsupportedPageSize => SysError::InvalidValue,
            VSpaceError::InvalidAsid => SysError::InvalidValue,
            VSpaceError::AllocationFailed => SysError::OutOfMemory,
        }
    }
}
