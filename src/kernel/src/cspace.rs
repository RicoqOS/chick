//! CSpace lookup operations for capability resolution.

use core::marker::PhantomData;
use core::ptr::NonNull;

use crate::error::{Result, SysError};
use crate::objects::capability::ObjType;
use crate::objects::cnode::{CNODE_DEPTH, CNodeCap, CNodeEntry};
use crate::objects::tcb::Tcb;

// 64 bits system.
const WORD_BITS: usize = 64;
const WORD_RADIX: usize = 6;

#[derive(Debug, Clone, Copy)]
pub struct ResolveResult<'a> {
    pub slot: &'a CNodeEntry,
    pub bits_remaining: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CSpace<'a> {
    root: NonNull<CNodeEntry>,
    _marker: PhantomData<&'a CNodeEntry>,
}

impl<'a> CSpace<'a> {
    /// Create a new [`CSpace`] from a root [`CNodeEntry`] entry.
    #[inline]
    pub fn new(root_entry: &'a CNodeEntry) -> Result<Self> {
        if root_entry.get().cap_type != ObjType::CNode {
            return Err(SysError::CSpaceNotFound);
        }

        Ok(Self {
            root: NonNull::from(root_entry),
            _marker: PhantomData,
        })
    }

    /// Create a [`CSpace`] from a [`Tcb`]'s cspace_root.
    #[inline]
    pub fn from_tcb(tcb: &'a Tcb) -> Result<Self> {
        Self::new(&tcb.cspace_root)
    }

    /// Lookup a capability by its pointer, resolving all bits.
    #[inline]
    pub fn lookup(&self, cptr: usize) -> Result<&'a CNodeEntry> {
        self.lookup_with_depth(cptr, CNODE_DEPTH)
    }

    /// Lookup a capability with a specific bit depth.
    #[inline]
    pub fn lookup_with_depth(
        &self,
        cptr: usize,
        depth: usize,
    ) -> Result<&'a CNodeEntry> {
        if depth < 1 || depth > CNODE_DEPTH {
            return Err(SysError::InvalidValue);
        }

        let res = self.resolve(cptr, depth)?;

        if res.bits_remaining != 0 {
            return Err(SysError::LookupError);
        }

        Ok(res.slot)
    }

    /// Resolve address bits, returning the slot and remaining bits.
    #[inline]
    pub fn resolve(
        &self,
        cptr: usize,
        n_bits: usize,
    ) -> Result<ResolveResult<'a>> {
        self.resolve_internal(cptr, n_bits)
    }

    #[inline]
    fn resolve_internal(
        &self,
        cptr: usize,
        mut n_bits: usize,
    ) -> Result<ResolveResult<'a>> {
        let mut current = self.root;

        loop {
            let entry = unsafe { current.as_ref() };

            let cap = CNodeCap::try_from_slot(entry)
                .ok_or(SysError::CSpaceNotFound)?;

            let radix_bits = cap.radix_bits();
            let guard_bits = cap.guard_bits();
            let level_bits = radix_bits + guard_bits;

            debug_assert!(level_bits != 0, "CNode must resolve bits");

            // Extract and verify guard.
            let shift = n_bits.wrapping_sub(guard_bits) & mask!(WORD_RADIX);
            let guard = if guard_bits == 0 {
                0
            } else {
                (cptr >> shift) & mask!(guard_bits)
            };

            if guard_bits > n_bits || guard != cap.guard() {
                return Err(SysError::LookupError);
            }

            if level_bits > n_bits {
                return Err(SysError::LookupError);
            }

            // Calculate slot index.
            let index = if radix_bits == 0 {
                0
            } else {
                (cptr >> (n_bits - level_bits)) & mask!(radix_bits)
            };

            let slot = cap.as_object().get(index).ok_or(SysError::SlotEmpty)?;

            // Terminal case: all bits resolved at this level.
            if n_bits == level_bits {
                return Ok(ResolveResult {
                    slot,
                    bits_remaining: 0,
                });
            }

            n_bits -= level_bits;

            // Check if we should continue traversing.
            if slot.get().cap_type != ObjType::CNode {
                return Ok(ResolveResult {
                    slot,
                    bits_remaining: n_bits,
                });
            }

            current = NonNull::from(slot);
        }
    }
}
