//! Untyped memory objects and retype operations.

use crate::error::{Result, SysError};
use crate::objects::capability::*;
use crate::objects::cnode::{CNODE_ENTRY_BIT_SZ, CNodeEntry, CNodeObj};
use crate::objects::nullcap::NullCap;
use crate::objects::tcb::Tcb;
use crate::{alignup, mask};

#[derive(Debug)]
pub struct UntypedObj {}

impl CapRef<'_, UntypedObj> {
    pub const ADDR_MASK: usize = mask!(Self::MIN_BIT_SIZE);
    pub const MIN_BIT_SIZE: usize = 4;

    pub const fn mint(paddr: usize, bit_sz: usize, is_device: bool) -> CapRaw {
        let mut capraw = CapRaw::default();
        capraw.paddr = paddr;
        capraw.arg1 = is_device as usize;
        capraw.arg2 = bit_sz & mask!(6);
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

        if bit_size > 64 {
            return Err(SysError::InvalidValue);
        }

        let count = slots.len();
        let obj_size = 1 << bit_size; // TODO: determine size by type.
        let tot_size = count * obj_size;
        let free_offset = alignup!(self.free_offset(), bit_size);

        if self.size() < tot_size + free_offset {
            return Err(SysError::InvalidValue);
        }

        for (i, slot) in slots.iter().enumerate() {
            let addr =
                self.paddr().as_u64() as usize + free_offset + i * obj_size;
            let cap = match obj_type {
                ObjType::Untyped => {
                    CapRef::<UntypedObj>::mint(addr, bit_size, self.is_device())
                },
                ObjType::CNode => {
                    let radix_sz = bit_size - CNODE_ENTRY_BIT_SZ;
                    CapRef::<CNodeObj>::mint(
                        addr,
                        radix_sz,
                        64 - radix_sz,
                        0,
                        slot.get().rights,
                    )
                },
                _ => return Err(SysError::InvalidValue),
            };

            slot.set(cap);

            if obj_type == ObjType::Untyped {
                CapRef::<UntypedObj>::mint(addr, obj_size, self.is_device());
            }
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
