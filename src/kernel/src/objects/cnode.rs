use core::cell::Cell;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::slice;

use crate::error::{Result as SysResult, SysError};
use crate::objects::capability::{CapRaw, CapRef, CapRights, ObjType};
use crate::objects::traits::KernelObject;

/// Maximum logical depth of [`CSpace`] (number of cap address bits).
pub const CNODE_DEPTH: usize = 32;
pub const CNODE_ENTRY_SZ: usize = size_of::<CNodeEntry>().next_power_of_two();
pub const CNODE_ENTRY_BIT_SZ: usize = CNODE_ENTRY_SZ.trailing_zeros() as usize;

pub type CNodeObj = [CNodeEntry];

impl CNodeEntry {
    /// Insert `dst` after `src` on MDB list.
    pub fn mdb_insert_after(src: &CNodeEntry, dst: &CNodeEntry) {
        let mut src_raw = src.get();
        let mut dst_raw = dst.get();
        let orig_next = src_raw.mdb_next;

        // Fix dst pointers.
        dst_raw.mdb_prev = Some(NonNull::from(src));
        dst_raw.mdb_next = orig_next;
        dst.set(dst_raw);

        // Update next prev.
        if let Some(next_ptr) = orig_next {
            unsafe {
                let next = next_ptr.as_ref();
                let mut next_raw = next.get();
                next_raw.mdb_prev = Some(NonNull::from(dst));
                next.set(next_raw);
            }
        }

        // Update next src.
        src_raw.mdb_next = Some(NonNull::from(dst));
        src.set(src_raw);
    }

    /// Remove current entry from MDB.
    pub fn mdb_remove(&self) {
        let mut self_raw = self.get();
        let prev = self_raw.mdb_prev;
        let next = self_raw.mdb_next;

        if let Some(prev_ptr) = prev {
            unsafe {
                let prev_entry = prev_ptr.as_ref();
                let mut prev_raw = prev_entry.get();
                prev_raw.mdb_next = next;
                prev_entry.set(prev_raw);
            }
        }

        if let Some(next_ptr) = next {
            unsafe {
                let next_entry = next_ptr.as_ref();
                let mut next_raw = next_entry.get();
                next_raw.mdb_prev = prev;
                next_entry.set(next_raw);
            }
        }

        self_raw.mdb_prev = None;
        self_raw.mdb_next = None;
        self.set(self_raw);
    }

    /// Revoke all rights to next cap objects in MDB chain.
    pub fn revoke(&self) {
        let mut cur = self.get().mdb_next;
        while let Some(ptr) = cur {
            unsafe {
                let entry = ptr.as_ref();
                let mut raw = entry.get();
                cur = raw.mdb_next;
                // Erase capability.
                raw.cap_type = ObjType::NullObj;
                raw.rights = CapRights::NONE;
                raw.paddr = 0;
                raw.arg1 = 0;
                raw.arg2 = 0;
                entry.set(raw);
                // Remove from chain.
                entry.mdb_remove();
            }
        }
    }
}

pub type CNodeCap<'a> = CapRef<'a, CNodeObj>;

impl CNodeCap<'_> {
    const GUARD_SZ_BITS: usize = 6;
    const GUARD_SZ_OFFSET: usize = 0;
    const RADIX_SZ_BITS: usize = 6;
    const RADIX_SZ_OFFSET: usize = Self::GUARD_SZ_OFFSET + Self::GUARD_SZ_BITS;

    const fn mask(bits: usize) -> usize {
        if bits >= core::mem::size_of::<usize>() * 8 {
            usize::MAX
        } else {
            (1usize << bits) - 1
        }
    }

    pub fn try_from_slot(slot: &'static CNodeEntry) -> Option<Self> {
        if slot.get().cap_type == ObjType::CNode {
            Some(Self {
                raw: slot,
                cap_type: PhantomData,
            })
        } else {
            None
        }
    }

    fn get_raw(&self) -> CapRaw {
        self.raw.0.get()
    }

    pub fn as_object(&self) -> &'static CNodeObj {
        let raw = self.get_raw();
        let size = 1usize << self.radix_bits();
        unsafe { slice::from_raw_parts(raw.paddr as *const CNodeEntry, size) }
    }

    pub fn as_object_mut(&self) -> &'static mut CNodeObj {
        let raw = self.get_raw();
        let size = 1usize << self.radix_bits();
        unsafe { slice::from_raw_parts_mut(raw.paddr as *mut CNodeEntry, size) }
    }

    pub fn guard_bits(&self) -> usize {
        let arg1 = self.get_raw().arg1;
        (arg1 >> Self::GUARD_SZ_OFFSET) & Self::mask(Self::GUARD_SZ_BITS)
    }

    pub fn radix_bits(&self) -> usize {
        let arg1 = self.get_raw().arg1;
        (arg1 >> Self::RADIX_SZ_OFFSET) & Self::mask(Self::RADIX_SZ_BITS)
    }

    pub fn size(&self) -> usize {
        1usize << self.radix_bits()
    }

    pub fn guard(&self) -> usize {
        // We consider arg2 contains a prepositioned guard.
        self.get_raw().arg2
    }

    /// Creates a [`ObjType::CNode`] cap from an array of slots + meta.
    pub const fn mint(
        addr: usize,
        radix_bits: usize,
        guard_bits: usize,
        guard: usize,
        rights: CapRights,
    ) -> CapRaw {
        let guard_sz = guard_bits & Self::mask(Self::GUARD_SZ_BITS);
        let radix_sz = radix_bits & Self::mask(Self::RADIX_SZ_BITS);

        assert!(radix_sz + guard_sz <= CNODE_DEPTH);

        let arg1 = (guard_sz << Self::GUARD_SZ_OFFSET) |
            (radix_sz << Self::RADIX_SZ_OFFSET);

        let mut capraw = CapRaw::default_with_type(ObjType::CNode);
        capraw.paddr = addr;
        capraw.arg1 = arg1;
        capraw.arg2 = guard;
        capraw.rights = rights;
        capraw
    }

    pub fn init(&self) {
        let node = self.as_object_mut();
        for slot in node.iter() {
            slot.set(CapRaw::default());
        }
    }
}

impl<'a, T: ?Sized + KernelObject> core::convert::TryFrom<&'a CNodeEntry>
    for CapRef<'a, T>
{
    type Error = SysError;

    fn try_from(value: &'a CNodeEntry) -> SysResult<Self> {
        if T::OBJ_TYPE != value.get().cap_type {
            Err(Self::Error::CapabilityTypeError)
        } else {
            Ok(Self {
                raw: value,
                cap_type: PhantomData,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct CNodeEntry(Cell<CapRaw>);

impl CNodeEntry {
    pub const fn new() -> Self {
        Self(Cell::new(CapRaw {
            arg1: 0,
            arg2: 0,
            paddr: 0,
            cap_type: ObjType::NullObj,
            rights: CapRights::NONE,
            mdb_prev: None,
            mdb_next: None,
        }))
    }

    pub fn get(&self) -> CapRaw {
        self.0.get()
    }

    pub fn set(&self, cap: CapRaw) {
        self.0.set(cap);
    }

    pub fn is_null(&self) -> bool {
        self.get().cap_type == ObjType::NullObj
    }
}
