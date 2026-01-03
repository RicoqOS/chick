//! Capability nodes with MDB.
use core::cell::Cell;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::slice;

use vstd::prelude::*;

use crate::error::{Result as SysResult, SysError};
use crate::objects::traits::KernelObject;
use crate::objects::{CapRaw, CapRef, CapRights, ObjType};

verus! {

/// Maximum logical depth of [`CSpace`] (number of cap address bits).
pub const CNODE_DEPTH: usize = 32;
pub const CNODE_ENTRY_SZ: usize = size_of::<CNodeEntry>().next_power_of_two();
pub const CNODE_ENTRY_BIT_SZ: usize = CNODE_ENTRY_SZ.trailing_zeros() as usize;

pub type CNodeObj = [CNodeEntry];

/// Representation of the MDB chain.
pub ghost struct MdbChainGhost {
    /// Sequence of entries in the chain (in order).
    pub entries: Seq<*const CNodeEntry>,
}

impl MdbChainGhost {
    pub open spec fn well_formed(self) -> bool {
        &&& self.entries.len() >= 0
        // First element has no prev.
        &&& (self.entries.len() > 0 ==> self.entry_at(
            0,
        ).mdb_prev.is_none())
        // Last element has no next.
        &&& (self.entries.len() > 0 ==> self.entry_at(
            self.entries.len() as int - 1,
        ).mdb_next.is_none())
        // Adjacent elements are properly linked.
        &&& forall|i: int| 0 <= i < self.entries.len() as int - 1 ==> self.adjacent_linked(i as nat)
    }

    pub open spec fn entry_at(self, i: int) -> CapRaw;

    pub open spec fn adjacent_linked(self, i: nat) -> bool {
        let curr = self.entry_at(i as int);
        let next = self.entry_at(i as int + 1);
        &&& curr.mdb_next.is_some()
        &&& next.mdb_prev.is_some()
        // curr.next points to next entry.
        &&& curr.mdb_next.unwrap().as_ptr() == self.entries[i as int
            + 1]
        // next.prev points to curr entry.
        &&& next.mdb_prev.unwrap().as_ptr() == self.entries[i as int]
    }

    pub open spec fn acyclic(self) -> bool {
        forall|i: int, j: int|
            0 <= i < j < self.entries.len() as int ==> self.entries[i] != self.entries[j]
    }
}

#[derive(Debug, Clone)]
pub struct CNodeEntry(Cell<CapRaw>);

impl CNodeEntry {
    /// Create a new null [`CNodeEntry`].
    pub const fn new() -> Self {
        Self(
            Cell::new(
                CapRaw {
                    arg1: 0,
                    arg2: 0,
                    paddr: 0,
                    cap_type: ObjType::NullObj,
                    rights: CapRights::NONE,
                    mdb_prev: None,
                    mdb_next: None,
                },
            ),
        )
    }

    pub closed spec fn view(self) -> CapRaw;

    pub fn get(&self) -> (result: CapRaw)
        ensures
            result == self.view(),
    {
        self.0.get()
    }

    /// Set the raw capability.
    pub fn set(&self, cap: CapRaw)
        ensures
            self.view() == cap,
    {
        self.0.set(cap);
    }

    pub fn is_null(&self) -> (result: bool)
        ensures
            result == (self.view().cap_type == ObjType::NullObj),
    {
        self.get().cap_type == ObjType::NullObj
    }

    pub proof fn new_is_null()
        ensures
            Self::new().view().cap_type == ObjType::NullObj,
    {
    }
}

impl CNodeEntry {
    /// Entry has no MDB links.
    pub open spec fn mdb_isolated(self) -> bool {
        self.view().mdb_prev.is_none() && self.view().mdb_next.is_none()
    }

    /// Insert `dst` after `src` on MDB list.
    pub fn mdb_insert_after(src: &CNodeEntry, dst: &CNodeEntry)
        requires
            dst.mdb_isolated(),
        ensures
            dst.view().mdb_prev.is_some(),
            src.view().mdb_next.is_some(),
    {
        let mut src_raw = src.get();
        let mut dst_raw = dst.get();
        let orig_next = src_raw.mdb_next;

        // Fix dst pointers.
        dst_raw.mdb_prev = Some(NonNull::from(src));
        dst_raw.mdb_next = orig_next;
        dst.set(dst_raw);

        proof {
            // After setting, dst has prev pointing to src.
            assert(dst.view().mdb_prev.is_some());
        }

        if let Some(next_ptr) = orig_next {
            unsafe {
                let next = next_ptr.as_ref();
                let mut next_raw = next.get();
                next_raw.mdb_prev = Some(NonNull::from(dst));
                next.set(next_raw);
            }
        }

        src_raw.mdb_next = Some(NonNull::from(dst));
        src.set(src_raw);

        proof {
            assert(src.view().mdb_next.is_some());
        }
    }

    pub open spec fn post_mdb_remove(self) -> bool {
        self.mdb_isolated()
    }

    /// Remove current entry from MDB.
    pub fn mdb_remove(&self)
        ensures
            self.mdb_isolated(),
            self.view().mdb_prev.is_none(),
            self.view().mdb_next.is_none(),
    {
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

        proof {
            assert(self.view().mdb_prev.is_none());
            assert(self.view().mdb_next.is_none());
        }
    }

    pub open spec fn is_nullified(raw: CapRaw) -> bool {
        &&& raw.cap_type == ObjType::NullObj
        &&& raw.rights == CapRights::NONE
        &&& raw.paddr == 0
        &&& raw.arg1 == 0
        &&& raw.arg2 == 0
    }

    /// Revoke all rights to next cap objects in MDB chain.
    pub fn revoke(&self)
        ensures
            self.view().mdb_next.is_none(),
    {
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

                proof {
                    assert(Self::is_nullified(entry.view()));
                }

                // Remove from chain.
                entry.mdb_remove();

                proof {
                    assert(entry.mdb_isolated());
                }
            }
        }
    }
}

pub type CNodeCap<'a> = CapRef<'a, CNodeObj>;

impl CNodeCap<'_> {
    const GUARD_BITS: usize = 6;
    const GUARD_OFFSET: usize = 0;
    // 64 bits systems.
    const RADIX_BITS: usize = 6;
    const RADIX_OFFSET: usize = Self::GUARD_OFFSET + Self::GUARD_BITS;

    pub open spec fn mask_spec(bits: usize) -> usize {
        if bits >= usize::BITS as usize {
            usize::MAX
        } else {
            (1usize << bits) - 1
        }
    }

    pub proof fn mask_bounded(bits: usize)
        requires
            bits < usize::BITS as usize,
        ensures
            Self::mask_spec(bits) == (1usize << bits) - 1,
            Self::mask_spec(bits) < (1usize << bits),
    {
    }

    pub proof fn mask_zero()
        ensures
            Self::mask_spec(0) == 0,
    {
    }

    const fn mask(bits: usize) -> (result: usize)
        ensures
            result == Self::mask_spec(bits),
            bits < usize::BITS as usize ==> result == (1usize << bits) - 1,
            bits >= usize::BITS as usize ==> result == usize::MAX,
    {
        if bits >= core::mem::size_of::<usize>() * 8 {
            usize::MAX
        } else {
            (1usize << bits) - 1
        }
    }

    /// Try to create a [`CNodeCap`] from a `slot`.
    pub fn try_from_slot(slot: &'static CNodeEntry) -> (result: Option<Self>)
        ensures
            result.is_some() ==> slot.view().cap_type == ObjType::CNode,
            result.is_none() ==> slot.view().cap_type != ObjType::CNode,
    {
        if slot.get().cap_type == ObjType::CNode {
            Some(Self { raw: slot, cap_type: PhantomData })
        } else {
            None
        }
    }

    fn get_raw(&self) -> CapRaw {
        self.raw.0.get()
    }

    pub fn guard_bits(&self) -> (result: usize)
        ensures
            result <= Self::mask_spec(Self::GUARD_BITS),
            result < 64,
    {
        let arg1 = self.get_raw().arg1;
        (arg1 >> Self::GUARD_OFFSET) & Self::mask(Self::GUARD_BITS)
    }

    pub fn radix_bits(&self) -> (result: usize)
        ensures
            result <= Self::mask_spec(Self::RADIX_BITS),
            result < 64,
    {
        let arg1 = self.get_raw().arg1;
        (arg1 >> Self::RADIX_OFFSET) & Self::mask(Self::RADIX_BITS)
    }

    /// Get the number of entries of current [`CNode`].
    pub fn size(&self) -> (result: usize)
        ensures
            result == 1usize << self.radix_bits(),
            result >= 1,  // At least one entry.
            self.radix_bits() < 64 ==> result == (1usize << self.radix_bits()),
    {
        proof {
            // radix_bits is bounded, so shift is safe.
            assert(self.radix_bits() < 64);
        }
        1usize << self.radix_bits()
    }

    pub fn guard(&self) -> usize {
        // We consider arg2 contains a prepositioned guard.
        self.get_raw().arg2
    }

    pub open spec fn mint_valid(radix_bits: usize, guard_bits: usize) -> bool {
        radix_bits + guard_bits <= CNODE_DEPTH
    }

    pub proof fn mint_depth_valid(radix_bits: usize, guard_bits: usize)
        requires
            Self::mint_valid(radix_bits, guard_bits),
        ensures
            radix_bits + guard_bits <= 32,
    {
    }

    /// Creates a [`ObjType::CNode`] cap from an array of slots + meta.
    pub const fn mint(
        addr: usize,
        radix_bits: usize,
        guard_bits: usize,
        guard: usize,
        rights: CapRights,
    ) -> (result: CapRaw)
        requires
            Self::mint_valid(radix_bits, guard_bits),
            (radix_bits & Self::mask_spec(Self::RADIX_BITS)) + (guard_bits & Self::mask_spec(
                Self::GUARD_BITS,
            )) <= CNODE_DEPTH,
        ensures
            result.cap_type == ObjType::CNode,
            result.paddr == addr,
            result.arg2 == guard,
            result.rights == rights,
            // The encoded radix_bits can be extracted correctly.
            ((result.arg1 >> Self::RADIX_OFFSET) & Self::mask_spec(Self::RADIX_BITS)) == (radix_bits
                & Self::mask_spec(Self::RADIX_BITS)),
            // The encoded guard_bits can be extracted correctly.
            ((result.arg1 >> Self::GUARD_OFFSET) & Self::mask_spec(Self::GUARD_BITS)) == (guard_bits
                & Self::mask_spec(Self::GUARD_BITS)),
    {
        let guard_sz = guard_bits & Self::mask(Self::GUARD_BITS);
        let radix_sz = radix_bits & Self::mask(Self::RADIX_BITS);

        assert!(radix_sz + guard_sz <= CNODE_DEPTH);

        let arg1 = (guard_sz << Self::GUARD_OFFSET) | (radix_sz << Self::RADIX_OFFSET);

        let mut capraw = CapRaw::default_with_type(ObjType::CNode);
        capraw.paddr = addr;
        capraw.arg1 = arg1;
        capraw.arg2 = guard;
        capraw.rights = rights;
        capraw
    }

    pub proof fn mint_radix_roundtrip(
        addr: usize,
        radix_bits: usize,
        guard_bits: usize,
        guard: usize,
        rights: CapRights,
    )
        requires
            Self::mint_valid(radix_bits, guard_bits),
            radix_bits < 64,
            guard_bits < 64,
        ensures
            ({
                let cap = Self::mint(addr, radix_bits, guard_bits, guard, rights);
                let extracted = (cap.arg1 >> Self::RADIX_OFFSET) & Self::mask_spec(
                    Self::RADIX_BITS,
                );
                extracted == (radix_bits & Self::mask_spec(Self::RADIX_BITS))
            }),
    {
    }

    pub proof fn mint_guard_roundtrip(
        addr: usize,
        radix_bits: usize,
        guard_bits: usize,
        guard: usize,
        rights: CapRights,
    )
        requires
            Self::mint_valid(radix_bits, guard_bits),
            radix_bits < 64,
            guard_bits < 64,
        ensures
            ({
                let cap = Self::mint(addr, radix_bits, guard_bits, guard, rights);
                let extracted = (cap.arg1 >> Self::GUARD_OFFSET) & Self::mask_spec(
                    Self::GUARD_BITS,
                );
                extracted == (guard_bits & Self::mask_spec(Self::GUARD_BITS))
            }),
    {
    }

    /// Get the object as a slice
    ///
    /// SAFETY: requires valid pointer.
    pub fn as_object(&self) -> (result: &'static CNodeObj)
        requires
            self.raw.view().paddr != 0,

        ensures
            result.len() == 1usize << self.radix_bits(),
    {
        let raw = self.get_raw();
        let size = 1usize << self.radix_bits();
        unsafe { slice::from_raw_parts(raw.paddr as *const CNodeEntry, size) }
    }

    /// Get the object as a mutable slice.
    ///
    /// SAFETY: requires valid pointer.
    pub fn as_object_mut(&self) -> (result: &'static mut CNodeObj)
        requires
            self.raw.view().paddr != 0,
            self.raw.view().rights.includes(CapRights::WRITE),

        ensures
            result.len() == 1usize << self.radix_bits(),
    {
        let raw = self.get_raw();
        let size = 1usize << self.radix_bits();
        unsafe { slice::from_raw_parts_mut(raw.paddr as *mut CNodeEntry, size) }
    }

    pub fn init(&self)
        requires
            self.raw.view().paddr != 0,
            self.raw.view().rights.includes(CapRights::WRITE),
        ensures
            forall|i: int|
                0 <= i < self.size() as int ==> self.as_object()[i].view().cap_type
                    == ObjType::NullObj,
    {
        let node = self.as_object_mut();
        let mut idx: usize = 0;

        while idx < node.len()
            invariant
                idx <= node.len(),
                // All processed slots are null.
                forall|i: int| 0 <= i < idx as int ==> node[i].view().cap_type == ObjType::NullObj,
        {
            node[idx].set(CapRaw::default());
            proof {
                assert(node[idx].view().cap_type == ObjType::NullObj);
            }
            idx += 1;
        }
    }
}

impl<'a, T: ?Sized + KernelObject> core::convert::TryFrom<&'a CNodeEntry> for CapRef<'a, T> {
    type Error = SysError;

    /// Convert a [`CNodeEntry`] to a typed [`CapRef`].
    fn try_from(value: &'a CNodeEntry) -> (result: SysResult<Self>)
        ensures
            result.is_ok() ==> T::OBJ_TYPE == value.view().cap_type,
            result.is_err() ==> T::OBJ_TYPE != value.view().cap_type,
    {
        if T::OBJ_TYPE != value.get().cap_type {
            Err(Self::Error::CapabilityTypeError)
        } else {
            Ok(Self { raw: value, cap_type: PhantomData })
        }
    }
}

pub proof fn cnode_depth_value()
    ensures
        CNODE_DEPTH == 32,
{
}

pub proof fn radix_guard_bounded(radix: usize, guard: usize)
    requires
        radix <= 63, // max 6 bits.
        guard <= 63, // max 6 bits.
        radix + guard <= CNODE_DEPTH,
    ensures
        radix + guard <= 32,
{
}

pub open spec fn cnode_cap_well_formed(cap: CapRaw) -> bool {
    let guard_bits = (cap.arg1 >> CNodeCap::GUARD_OFFSET) & CNodeCap::mask_spec(
        CNodeCap::GUARD_BITS,
    );
    let radix_bits = (cap.arg1 >> CNodeCap::RADIX_OFFSET) & CNodeCap::mask_spec(
        CNodeCap::RADIX_BITS,
    );

    &&& cap.cap_type == ObjType::CNode
    &&& guard_bits + radix_bits <= CNODE_DEPTH
    &&& cap.paddr != 0

}

pub proof fn mint_well_formed(
    addr: usize,
    radix_bits: usize,
    guard_bits: usize,
    guard: usize,
    rights: CapRights,
)
    requires
        CNodeCap::mint_valid(radix_bits, guard_bits),
        addr != 0,
        radix_bits < 64,
        guard_bits < 64,
    ensures
        cnode_cap_well_formed(CNodeCap::mint(addr, radix_bits, guard_bits, guard, rights)),
{
}

} // verus!
