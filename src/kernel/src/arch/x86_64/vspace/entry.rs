//! Page table entry definitions for x86-64.

use x86_64::PhysAddr;

use crate::bit;
use crate::vspace::{CachePolicy, VMAttributes, VMRights};

const PRESENT: u64 = bit!(0);
const WRITABLE: u64 = bit!(1);
const USER: u64 = bit!(2);
const WRITE_THROUGH: u64 = bit!(3);
const CACHE_DISABLE: u64 = bit!(4);
const ACCESSED: u64 = bit!(5);
const DIRTY: u64 = bit!(6);
const HUGE_PAGE: u64 = bit!(7);
const GLOBAL: u64 = bit!(8);
const NO_EXECUTE: u64 = bit!(63);

const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

/// Common trait for all page table entries.
pub trait PageTableEntry: Copy + Clone + Sized {
    /// Create an invalid (not present) entry.
    fn invalid() -> Self;

    /// Check if the entry is present/valid.
    fn is_present(&self) -> bool;

    /// Check if this is a table entry (points to next level).
    fn is_table(&self) -> bool;

    /// Check if this is a page entry (huge/large page).
    fn is_page(&self) -> bool;

    /// Get the physical address from the entry.
    fn paddr(&self) -> PhysAddr;

    /// Get the raw entry value.
    fn raw(&self) -> u64;

    /// Create from raw value.
    fn from_raw(raw: u64) -> Self;
}

/// PML4 Entry (always points to PDPT).
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct Pml4e(u64);

/// PDPT Entry (points to PD or 1GB page).
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct Pdpte(u64);

/// Page Directory Entry (points to PT or 2MB page).
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct Pde(u64);

/// Page Table Entry (points to 4KB frame).
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct Pte(u64);

fn build_flags(attr: &VMAttributes) -> u64 {
    let mut flags = PRESENT;

    if attr.rights.contains(VMRights::WRITE) {
        flags |= WRITABLE;
    }
    if attr.user {
        flags |= USER;
    }
    if attr.global {
        flags |= GLOBAL;
    }
    if !attr.rights.contains(VMRights::EXECUTE) {
        flags |= NO_EXECUTE;
    }

    // Cache policy.
    match attr.cache {
        CachePolicy::WriteBack => {},
        CachePolicy::WriteThrough => flags |= WRITE_THROUGH,
        CachePolicy::Uncacheable => flags |= CACHE_DISABLE,
        CachePolicy::WriteCombining => flags |= WRITE_THROUGH | CACHE_DISABLE,
    }

    flags
}

impl Pml4e {
    /// Create a PML4 entry pointing to a PDPT.
    pub const fn table(paddr: PhysAddr, attr: VMAttributes) -> Self {
        let flags = PRESENT |
            if attr.rights.contains(VMRights::WRITE) {
                WRITABLE
            } else {
                0
            } |
            if attr.user { USER } else { 0 };
        Self((paddr.as_u64() & ADDR_MASK) | flags)
    }

    /// Create a table entry (common case).
    pub fn new_table(paddr: PhysAddr) -> Self {
        Self((paddr.as_u64() & ADDR_MASK) | PRESENT | WRITABLE | USER)
    }
}

impl PageTableEntry for Pml4e {
    fn invalid() -> Self {
        Self(0)
    }

    fn is_present(&self) -> bool {
        self.0 & PRESENT != 0
    }

    fn is_table(&self) -> bool {
        self.is_present() // Pml4e is always a table entry.
    }

    fn is_page(&self) -> bool {
        false // PML4 cannot map pages directly.
    }

    fn paddr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & ADDR_MASK)
    }

    fn raw(&self) -> u64 {
        self.0
    }

    fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

impl Pdpte {
    /// Create a PDPT entry pointing to a Page Directory.
    pub fn new_table(paddr: PhysAddr) -> Self {
        Self((paddr.as_u64() & ADDR_MASK) | PRESENT | WRITABLE | USER)
    }

    /// Create a 1GB huge page entry.
    pub fn new_huge_page(paddr: PhysAddr, attr: VMAttributes) -> Self {
        let flags = build_flags(&attr) | HUGE_PAGE;
        Self((paddr.as_u64() & ADDR_MASK) | flags)
    }
}

impl PageTableEntry for Pdpte {
    fn invalid() -> Self {
        Self(0)
    }

    fn is_present(&self) -> bool {
        self.0 & PRESENT != 0
    }

    fn is_table(&self) -> bool {
        self.is_present() && (self.0 & HUGE_PAGE == 0)
    }

    fn is_page(&self) -> bool {
        self.is_present() && (self.0 & HUGE_PAGE != 0)
    }

    fn paddr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & ADDR_MASK)
    }

    fn raw(&self) -> u64 {
        self.0
    }

    fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

impl Pde {
    /// Create a PD entry pointing to a Page Table.
    pub fn new_table(paddr: PhysAddr) -> Self {
        Self((paddr.as_u64() & ADDR_MASK) | PRESENT | WRITABLE | USER)
    }

    /// Create a 2MB large page entry.
    pub fn new_large_page(paddr: PhysAddr, attr: VMAttributes) -> Self {
        let flags = build_flags(&attr) | HUGE_PAGE;
        Self((paddr.as_u64() & ADDR_MASK) | flags)
    }
}

impl PageTableEntry for Pde {
    fn invalid() -> Self {
        Self(0)
    }

    fn is_present(&self) -> bool {
        self.0 & PRESENT != 0
    }

    fn is_table(&self) -> bool {
        self.is_present() && (self.0 & HUGE_PAGE == 0)
    }

    fn is_page(&self) -> bool {
        self.is_present() && (self.0 & HUGE_PAGE != 0)
    }

    fn paddr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & ADDR_MASK)
    }

    fn raw(&self) -> u64 {
        self.0
    }

    fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}

impl Pte {
    /// Create a 4KB page entry.
    pub fn new_page(paddr: PhysAddr, attr: VMAttributes) -> Self {
        let flags = build_flags(&attr);
        Self((paddr.as_u64() & ADDR_MASK) | flags)
    }
}

impl PageTableEntry for Pte {
    fn invalid() -> Self {
        Self(0)
    }

    fn is_present(&self) -> bool {
        self.0 & PRESENT != 0
    }

    fn is_table(&self) -> bool {
        false // PT entries never point to tables.
    }

    fn is_page(&self) -> bool {
        self.is_present()
    }

    fn paddr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & ADDR_MASK)
    }

    fn raw(&self) -> u64 {
        self.0
    }

    fn from_raw(raw: u64) -> Self {
        Self(raw)
    }
}
