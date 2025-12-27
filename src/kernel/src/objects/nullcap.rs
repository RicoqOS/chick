use core::marker::PhantomData;

use crate::objects::capability::{CapRaw, CapRef};
use crate::objects::tcb::Tcb;
use crate::objects::traits::KernelObject;

#[derive(Debug)]
pub enum NullObj {}

pub type NullCap<'a> = CapRef<'a, NullObj>;

impl<'a> CapRef<'a, NullObj> {
    pub const fn mint() -> CapRaw {
        CapRaw::default()
    }

    pub fn insert<T>(self, raw: CapRaw) -> CapRef<'a, T>
    where
        T: KernelObject + ?Sized,
    {
        debug_assert_eq!(T::OBJ_TYPE, raw.cap_type);
        self.raw.set(raw);

        CapRef {
            raw: self.raw,
            cap_type: PhantomData,
        }
    }

    pub fn insert_raw(&self, raw: CapRaw) {
        self.raw.set(raw);
    }

    pub fn identify(&self, tcb: &mut Tcb) -> usize {
        tcb.set_mr(Tcb::MR1, self.cap_type() as usize);
        1
    }
}
