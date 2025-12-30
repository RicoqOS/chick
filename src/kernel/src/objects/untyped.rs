//! Untyped memory objects and retype operations.

use crate::error::{Result, SysError};
use crate::objects::capability::*;
use crate::objects::cnode::{CNODE_ENTRY_BIT_SZ, CNodeEntry, CNodeObj};
use crate::objects::frame::{FrameObj, FrameSize};
use crate::objects::nullcap::NullCap;
use crate::objects::tcb::Tcb;
use crate::objects::vspace::VSpaceCap;
use crate::vspace::PAGE_BITS_4K;
use crate::{alignup, mask};

#[derive(Debug)]
pub struct UntypedObj {}

impl CapRef<'_, UntypedObj> {
    pub const ADDR_MASK: usize = mask!(Self::MIN_BIT_SIZE);
    pub const MIN_BIT_SIZE: usize = 4;

    pub const fn mint(paddr: usize, bit_sz: usize, is_device: bool) -> CapRaw {
        let mut capraw = CapRaw::default_with_type(ObjType::Untyped);
        capraw.paddr = paddr;
        capraw.arg1 = is_device as usize;
        capraw.arg2 = bit_sz & mask!(6);
        capraw.rights = CapRights::CONTROL;
        capraw
    }

    pub fn bit_size(&self) -> usize {
        self.raw.get().arg2 & mask!(6)
    }

    pub fn size(&self) -> usize {
        1 << self.bit_size()
    }

    pub fn free_offset(&self) -> usize {
        self.raw.get().arg2 >> 6
    }

    pub fn set_free_offset(&self, off: usize) {
        let mut cap = self.raw.get();
        cap.arg2 = cap.arg2 & mask!(6) | (off << 6);
        self.raw.set(cap);
    }

    pub fn is_device(&self) -> bool {
        self.raw.get().arg1 != 0
    }

    /// Calculate required alignment for an object type.
    fn object_alignment(obj_type: ObjType, bit_size: usize) -> usize {
        match obj_type {
            ObjType::Frame => bit_size,
            ObjType::VSpace => PAGE_BITS_4K,
            ObjType::CNode => {
                // CNode alignment depends on size.
                let entry_sz = CNODE_ENTRY_BIT_SZ;
                let radix = bit_size.saturating_sub(entry_sz);
                entry_sz + radix
            },
            ObjType::Tcb => 10, // TCBs are 1024-byte aligned.
            _ => bit_size,
        }
    }

    const fn object_size(obj_type: ObjType, user_bits: usize) -> Option<usize> {
        match obj_type {
            ObjType::Frame => match user_bits {
                12 | 21 | 30 => Some(1 << user_bits),
                _ => None,
            },
            ObjType::VSpace => Some(1 << PAGE_BITS_4K), // one page.
            ObjType::CNode => {
                if user_bits >= CNODE_ENTRY_BIT_SZ && user_bits <= 48 {
                    Some(1 << user_bits)
                } else {
                    None
                }
            },
            ObjType::Tcb => Some(1 << 10),
            ObjType::Untyped => {
                if user_bits >= Self::MIN_BIT_SIZE && user_bits <= 48 {
                    Some(1 << user_bits)
                } else {
                    None
                }
            },
            _ => None,
        }
    }

    /// Allocate slots objects of given type.
    pub fn retype(
        &self,
        obj_type: ObjType,
        bit_size: usize,
        slots: &[CNodeEntry],
    ) -> Result<()> {
        if slots.iter().any(|cap| NullCap::try_from(cap).is_err()) {
            return Err(SysError::SlotNotEmpty);
        }

        if self.is_device() {
            match obj_type {
                ObjType::Frame | ObjType::Untyped => {},
                _ => return Err(SysError::InvalidValue),
            }
        }

        let align_bits = Self::object_alignment(obj_type, bit_size);
        let obj_size = Self::object_size(obj_type, bit_size)
            .ok_or(SysError::InvalidValue)?;
        let count = slots.len();
        let tot_size =
            count.checked_mul(obj_size).ok_or(SysError::InvalidValue)?;
        let free_offset = alignup!(self.free_offset(), align_bits);

        let required = free_offset
            .checked_add(tot_size)
            .ok_or(SysError::InvalidValue)?;

        if self.size() < required {
            return Err(SysError::OutOfMemory);
        }

        let base_paddr = self.paddr().as_u64() as usize;
        for (i, slot) in slots.iter().enumerate() {
            let addr = base_paddr + free_offset + i * obj_size;
            let cap = match obj_type {
                ObjType::Untyped => {
                    CapRef::<UntypedObj>::mint(addr, bit_size, self.is_device())
                },
                ObjType::CNode => {
                    let radix_sz = bit_size.saturating_sub(CNODE_ENTRY_BIT_SZ);
                    let guard_bits = 64usize.saturating_sub(radix_sz);

                    // Zeroize CNode memory.
                    // SAFETY: We own this memory region via the untyped
                    // capability.
                    unsafe {
                        core::ptr::write_bytes(addr as *mut u8, 0, obj_size);
                    }

                    CapRef::<CNodeObj>::mint(
                        addr,
                        radix_sz,
                        guard_bits,
                        0,
                        CapRights::CONTROL,
                    )
                },
                ObjType::Frame => {
                    let size = FrameSize::from_bits(bit_size)
                        .ok_or(SysError::InvalidValue)?;
                    CapRef::<FrameObj>::mint(
                        addr,
                        size,
                        self.is_device(),
                        CapRights::READ | CapRights::WRITE,
                    )
                },
                ObjType::VSpace => {
                    // SAFETY: We own this memory region via the untyped
                    // capability.
                    unsafe {
                        core::ptr::write_bytes(addr as *mut u8, 0, obj_size);
                    }

                    // Allocate a new ASID.
                    // For now, use a simple counter based on address.
                    // TODO: Implement proper ASID pool management.
                    let asid = ((addr >> PAGE_BITS_4K) & 0xFFFF) as u16;
                    let asid = if asid == 0 { 1 } else { asid }; // ASID 0 is reserved.

                    VSpaceCap::mint(addr, asid, CapRights::CONTROL)
                },
                _ => return Err(SysError::InvalidValue),
            };

            slot.set(cap);
        }

        self.set_free_offset(free_offset + tot_size);
        Ok(())
    }

    pub fn identify(&self, tcb: &mut Tcb) -> usize {
        tcb.set_mr(Tcb::MR1, self.cap_type() as usize);
        tcb.set_mr(Tcb::MR2, self.paddr().as_u64() as usize);
        tcb.set_mr(Tcb::MR3, self.bit_size());
        tcb.set_mr(Tcb::MR4, self.is_device() as usize);
        tcb.set_mr(Tcb::MR5, self.free_offset());
        5
    }
}
