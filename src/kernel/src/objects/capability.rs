use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::objects::cnode::{CNODE_DEPTH, CNodeCap, CNodeEntry, CNodeObj};
use crate::objects::traits::KernelObject;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjType {
    #[default]
    NullObj = 0,
    Untyped = 1,
    CNode = 2,
    Tcb = 3,
    Frame = 4,
    Endpoint = 5,
    Reply = 6,
    Monitor = 7,
    Interrupt = 8,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct CapRights: u8 {
        const NONE    = 0b0000_0000;
        const READ    = 0b0000_0001;
        const WRITE   = 0b0000_0010;
        const EXECUTE = 0b0000_0100;
        const GRANT   = 0b0000_1000;
        const CONTROL = 0b0001_0000;
        const SEND    = 0b0010_0000;
        const RECEIVE = 0b0100_0000;
    }
}

/// Capability entry field definition.
#[derive(Debug, Clone, Copy)]
pub struct CapRef<'a, T: KernelObject + ?Sized> {
    pub raw: &'a CNodeEntry,
    pub cap_type: PhantomData<T>,
}

impl<'a, T: KernelObject + ?Sized> CapRef<'a, T> {
    pub fn cap_type(&self) -> ObjType {
        debug_assert_eq!(T::OBJ_TYPE, self.raw.get().cap_type);
        T::OBJ_TYPE
    }

    #[cfg(target_arch = "x86_64")]
    pub fn paddr(&self) -> x86_64::PhysAddr {
        x86_64::PhysAddr::new(self.raw.get().paddr as u64)
    }

    fn _retype<U: KernelObject + ?Sized>(self) -> CapRef<'a, U> {
        debug_assert_eq!(U::OBJ_TYPE, self.raw.get().cap_type);
        CapRef {
            raw: self.raw,
            cap_type: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(align(32))]
pub struct CapRaw {
    pub arg1: usize,
    pub arg2: usize,
    pub paddr: usize,
    pub cap_type: ObjType,
    pub rights: CapRights,
    pub mdb_prev: Option<NonNull<CNodeEntry>>,
    pub mdb_next: Option<NonNull<CNodeEntry>>,
}

impl CapRaw {
    /// New [`CapRaw`] default structure.
    pub const fn default() -> Self {
        Self {
            arg1: 0,
            arg2: 0,
            paddr: 0,
            cap_type: ObjType::NullObj,
            rights: CapRights::NONE,
            mdb_prev: None,
            mdb_next: None,
        }
    }

    /// Create a new [`CapRaw`] with custom [`ObjType`].
    pub const fn default_with_type(cap_type: ObjType) -> Self {
        let mut capraw = Self::default();
        capraw.cap_type = cap_type;
        capraw
    }
}

pub struct CSpace<'a>(pub &'a mut CNodeObj);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupError {
    DepthExceeded,
    GuardMismatch,
    SlotEmpty,
    NotCNode,
}

impl<'a> CSpace<'a> {
    /// Resolves a `cptr` (up to `CNODE_DEPTH` bits) to a CNode slot.
    pub fn lookup_slot(
        &self,
        cptr: usize,
    ) -> Result<&'a CNodeEntry, LookupError> {
        let mut depth = CNODE_DEPTH;
        let root_entry = unsafe {
            // SAFETY: We need to extend the lifetime from self's borrow to 'a
            // This is safe because CSpace<'a> guarantees the CNodeObj lives for
            // 'a
            &*(&self.0[0] as *const CNodeEntry)
        };
        let mut cur_cap =
            CNodeCap::try_from_slot(root_entry).ok_or(LookupError::NotCNode)?;
        let idx = cptr;

        loop {
            let radix = cur_cap.radix_bits();
            let guard_bits = cur_cap.guard_bits();
            let level_bits = radix + guard_bits;

            if level_bits > depth {
                return Err(LookupError::DepthExceeded);
            }

            // Position of bits to read starting from the MSB in the depth bits.
            // guard = bits [depth..  depth - guard_bits)
            // index = bits [depth - guard_bits.. depth - level_bits)
            let shift_guard = depth - guard_bits;
            let guard_mask = if guard_bits == 0 {
                0
            } else {
                (1usize << guard_bits).wrapping_sub(1)
            };
            let guard_val = (idx >> shift_guard) & guard_mask;

            if guard_bits > 0 &&
                guard_val != (cur_cap.guard() >> shift_guard) & guard_mask
            {
                return Err(LookupError::GuardMismatch);
            }

            let shift_index = depth - level_bits;
            let index_mask = (1usize << radix).wrapping_sub(1);
            let index = (idx >> shift_index) & index_mask;

            let node = cur_cap.as_object();
            let slot = node.get(index).ok_or(LookupError::SlotEmpty)?;

            if depth == level_bits {
                return Ok(slot);
            }

            let raw = slot.get();
            if raw.cap_type != ObjType::CNode {
                return Ok(slot);
            }

            // Go down to the child CNode.
            depth -= level_bits;
            cur_cap =
                CNodeCap::try_from_slot(slot).ok_or(LookupError::NotCNode)?;
        }
    }
}
