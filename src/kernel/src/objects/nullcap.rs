//! Null capabilities.

use core::marker::PhantomData;

use vstd::prelude::*;

use crate::objects::tcb::Tcb;
use crate::objects::traits::KernelObject;
use crate::objects::{CapRaw, CapRef};

verus! {

#[derive(Debug)]
pub enum NullObj {}

pub type NullCap<'a> = CapRef<'a, NullObj>;

pub open spec fn is_null_cap(raw: CapRaw) -> bool {
    raw == CapRaw::default()
}

pub open spec fn cap_type_matches<T: KernelObject + ?Sized>(raw: CapRaw) -> bool {
    T::OBJ_TYPE == raw.cap_type
}

impl<'a> CapRef<'a, NullObj> {
    /// Creates a new null [`CapRaw`] value.
    pub const fn mint() -> (result: CapRaw)
        ensures
            is_null_cap(result),
            result == CapRaw::default(),
    {
        CapRaw::default()
    }

    /// Inserts a capability into this null slot, transforming it into a
    /// capability reference of type `T`.
    pub fn insert<T: KernelObject + ?Sized>(self, raw: CapRaw) -> (o: CapRef<'a, T>)
        requires
            cap_type_matches::<T>(raw),
        ensures
            o.raw@ == raw,
    {
        self.raw.set(raw);
        CapRef { raw: self.raw, cap_type: PhantomData }
    }

    /// Inserts a raw capability value into this slot without type
    /// transformation.
    pub fn insert_raw(&self, raw: CapRaw)
        ensures
            self.raw@ == raw,
    {
        self.raw.set(raw);
    }

    /// Identifies capability type on `MR1`.
    pub fn identify(&self, tcb: &mut Tcb) -> (result: usize)
        requires
            tcb.is_valid(),
        ensures
            result == 1,
            tcb.get_mr(Tcb::MR1) == self.cap_type() as usize,
            // TCB remains valid after operation.
            tcb.is_valid(),
    {
        tcb.set_mr(Tcb::MR1, self.cap_type() as usize);
        1
    }
}

} // verus!
