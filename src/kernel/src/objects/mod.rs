//! seL4-like capabilities objects.

pub mod cnode;
pub mod endpoint;
pub mod frame;
pub mod nullcap;
pub mod tcb;
pub mod traits;
pub mod untyped;
pub mod vspace;

use core::marker::PhantomData;
use core::ptr::NonNull;

use vstd::prelude::*;

use crate::arch::PhysAddr;
use crate::objects::cnode::CNodeEntry;
use crate::objects::traits::KernelObject;

verus! {

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
    VSpace = 9,
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

impl CapRights {
    pub open spec fn includes(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub proof fn reflexive(self)
        ensures
            self.includes(self),
    {
    }

    pub proof fn transitive(a: Self, b: Self, c: Self)
        requires
            a.includes(b),
            b.includes(c),
        ensures
            a.includes(c),
    {
    }
}

/// Capability entry field definition.
#[derive(Debug, Clone, Copy)]
pub struct CapRef<'a, T: KernelObject + ?Sized> {
    pub raw: &'a CNodeEntry,
    pub cap_type: PhantomData<T>,
}

pub open spec fn cap_type_matches<T: KernelObject + ?Sized>(raw: CapRaw) -> bool {
    T::OBJ_TYPE == raw.cap_type
}

impl<T: KernelObject + ?Sized> CapRef<'_, T> {
    pub fn cap_type(&self) -> (result: ObjType)
        requires
            cap_type_matches::<T>(self.raw@),
        ensures
            result == T::OBJ_TYPE,
            result == self.raw@.cap_type,
    {
        T::OBJ_TYPE
    }

    pub fn paddr(&self) -> PhysAddr {
        PhysAddr::new(self.raw.get().paddr as u64)
    }

    /// Get the rights associated with this capability.
    pub fn rights(&self) -> CapRights {
        self.raw.get().rights
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

} // verus!
