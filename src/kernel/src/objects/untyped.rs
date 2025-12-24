use core::cell::Cell;

use crate::error::{Result, SysError};
use crate::objects::capacity::*;
use crate::objects::nullcap::NullCap;
use crate::objects::tcb::Tcb;
use crate::{alignup, mask};

#[derive(Debug)]
pub struct UntypedObj {}

pub type UntypedCap<'a> = CapRef<'a, UntypedObj>;

impl<'a> CapRef<'a, UntypedObj> {
    pub const ADDR_MASK: usize = mask!(Self::MIN_BIT_SIZE);
    pub const MIN_BIT_SIZE: usize = 4;

    pub const fn mint(addr: usize, bit_sz: usize, is_device: bool) -> CapRaw {
        CapRaw::default()
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

    // Allocate `slots.len()` objects of type `obj_type`. putting to `slots`
    //
    // `size`: for variable sized caps, `size` is the size of each new object.
    // ignored for constant sized objects.
    // `slots`: a range of slots to put new objects. need to check if empty
    pub fn retype(
        &self,
        obj_type: ObjType,
        bit_size: usize,
        slots: &[CNodeEntry],
    ) -> Result<()> {
        /*if slots.iter().any(|cap| NullCap::try_from(cap).is_err()) {
            return Err(SysError::SlotNotEmpty);
        }

        if bit_size > 64 {
            return Err(SysError::InvalidValue);
        }

        let count = slots.len();
        let obj_size = 1 << bit_size; //TODO: determine size by type;
        let tot_size = count * obj_size;
        let free_offset = alignup!(self.free_offset(), bit_size);

        if self.size() < tot_size + free_offset {
            return Err(SysError::InvalidValue);
        }

        for (i, slot) in slots.iter().enumerate() {
            let addr = self.paddr().0 + free_offset + i * obj_size;
            let cap = match obj_type {
                ObjType::Untyped => {
                    CapRef::<UntypedObj>::mint(addr, bit_size, self.is_device())
                },
                ObjType::CNode => {
                    let radix_sz = bit_size - CNODE_ENTRY_BIT_SZ;

                    CapRef::<CNodeObj>::mint(addr, radix_sz, 64 - radix_sz, 0)
                },
                ObjType::Tcb => CapRef::<Tcb>::mint(addr),
                // ObjType::Ram => CapRef::<RamObj>::mint(
                // addr,
                // true,
                // true,
                // bit_size,
                // self.is_device(),
                // ),
                // ObjType::VTable => CapRef::<VTableObj>::mint(addr),
                // ObjType::Endpoint => CapRef::<EndpointObj>::mint(addr, 0),
                _ => return Err(SysError::InvalidValue),
            };

            slot.set(cap);

            match obj_type {
                //                ObjType::NullObj => { unreachable!() },
                //                ObjType::Untyped => {
                // CapRef::<UntypedObj>::mint(addr, obj_size) },
                //ObjType::CNode => CNodeCap::try_from(slot).unwrap().init(),
                // ObjType::Tcb => TcbCap::try_from(slot).unwrap().init(),
                // ObjType::Ram => RamCap::try_from(slot).unwrap().init(),
                // ObjType::VTable => VTableCap::try_from(slot).unwrap().init(),
                // ObjType::Endpoint => {
                // EndpointCap::try_from(slot).unwrap().init()
                // },
                _ => {},
            }
        }
        self.set_free_offset(free_offset + tot_size);
        Ok(())*/
        Ok(())
    }
}
