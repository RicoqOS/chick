use core::cell::Cell;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::slice;

use crate::objects::traits::KernelObject;

/// Type d'objet noyau (fortement inspiré de seL4, mais réduit)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjType {
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
    cap_type: PhantomData<T>,
}

/// Maximum logical depth of [`CSpace`] (number of cap address bits).
pub const CNODE_DEPTH: usize = 32;

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
        if let Some(mut next_ptr) = orig_next {
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

        if let Some(mut prev_ptr) = prev {
            unsafe {
                let prev_entry = prev_ptr.as_ref();
                let mut prev_raw = prev_entry.get();
                prev_raw.mdb_next = next;
                prev_entry.set(prev_raw);
            }
        }

        if let Some(mut next_ptr) = next {
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
        while let Some(mut ptr) = cur {
            unsafe {
                let entry = ptr.as_ref();
                let mut raw = entry.get();
                cur = raw.mdb_next;
                // Erease capability.
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

#[derive(Clone, Copy, Debug)]
pub struct CNodeCap {
    pub slot: &'static CNodeEntry,
}

impl CNodeCap {
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
            Some(Self { slot })
        } else {
            None
        }
    }

    fn raw(&self) -> CapRaw {
        self.slot.get()
    }

    pub fn paddr(&self) -> usize {
        self.raw().paddr
    }

    pub fn as_object(&self) -> &'static CNodeObj {
        let raw = self.raw();
        let size = 1usize << self.radix_bits();
        unsafe { slice::from_raw_parts(raw.paddr as *const CNodeEntry, size) }
    }

    pub fn as_object_mut(&self) -> &'static mut CNodeObj {
        let raw = self.raw();
        let size = 1usize << self.radix_bits();
        unsafe { slice::from_raw_parts_mut(raw.paddr as *mut CNodeEntry, size) }
    }

    pub fn guard_bits(&self) -> usize {
        let arg1 = self.raw().arg1;
        (arg1 >> Self::GUARD_SZ_OFFSET) & Self::mask(Self::GUARD_SZ_BITS)
    }

    pub fn radix_bits(&self) -> usize {
        let arg1 = self.raw().arg1;
        (arg1 >> Self::RADIX_SZ_OFFSET) & Self::mask(Self::RADIX_SZ_BITS)
    }

    pub fn size(&self) -> usize {
        1usize << self.radix_bits()
    }

    pub fn guard(&self) -> usize {
        // We consider arg2 contains a prepositioned guard.
        self.raw().arg2
    }

    /// Creates a [`ObjType::CNode`] cap from an array of slots + meta.
    pub fn mint(
        base_addr: usize,
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

        CapRaw {
            arg1,
            arg2: guard,
            paddr: base_addr,
            cap_type: ObjType::CNode,
            rights,
            mdb_prev: None,
            mdb_next: None,
        }
    }

    pub fn init(&self) {
        let node = self.as_object_mut();
        for slot in node.iter() {
            slot.set(CapRaw::default());
        }
    }
}

pub struct CSpace {
    pub root: CNodeCap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookupError {
    DepthExceeded,
    GuardMismatch,
    SlotEmpty,
    NotCNode,
}

impl CSpace {
    /// Resolves a `cptr` (up to `CNODE_DEPTH` bits) to a CNode slot.
    ///
    /// Logical layout at the current level:
    /// - d = number of bits remaining to be interpreted
    /// - guard_bits = g, radix_bits = r
    ///  -> first take g guard bits, then r index bits
    ///  -> g + r <= d
    pub fn lookup_slot(
        &self,
        cptr: usize,
    ) -> Result<&'static CNodeEntry, LookupError> {
        let mut depth = CNODE_DEPTH;
        let mut cur_cap = self.root;
        let mut idx = cptr;

        loop {
            let radix = cur_cap.radix_bits();
            let guard_bits = cur_cap.guard_bits();
            let level_bits = radix + guard_bits;

            if level_bits > depth {
                return Err(LookupError::DepthExceeded);
            }

            // Position of bits to read starting from the MSB in the depth bits.
            // guard = bits [depth..depth - guard_bits)
            // index = bits [depth - guard_bits..depth - level_bits)
            let shift_guard = depth - guard_bits;
            let guard_mask = (1usize << guard_bits) - 1;
            let guard_val = (idx >> shift_guard) & guard_mask;

            if guard_bits > 0 &&
                guard_val != (cur_cap.guard() >> shift_guard & guard_mask)
            {
                return Err(LookupError::GuardMismatch);
            }

            let shift_index = depth - level_bits;
            let index_mask = (1usize << radix) - 1;
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
            cur_cap = CNodeCap { slot };
        }
    }
}
