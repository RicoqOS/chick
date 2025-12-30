//! Page table level definitions for x86-64 4-level paging.

use crate::arch::vspace::entry::{PDE, PDPTE, PML4E, PTE};
use crate::vspace::{
    Level, PAGE_BITS_1G, PAGE_BITS_2M, PAGE_BITS_4K, PageLevel, TableLevel,
    TopLevel,
};

#[derive(Copy, Clone, Debug)]
pub struct Pml4;

#[derive(Copy, Clone, Debug)]
pub struct Pdpt;

#[derive(Copy, Clone, Debug)]
pub struct PageDirectory;

#[derive(Copy, Clone, Debug)]
pub struct Pt;

#[derive(Copy, Clone, Debug)]
pub struct Frame;

impl Level for Pml4 {
    const LEVEL: usize = 4;
}

impl Level for Pdpt {
    const LEVEL: usize = 3;
}

impl Level for PageDirectory {
    const LEVEL: usize = 2;
}

impl Level for Pt {
    const LEVEL: usize = 1;
}

impl Level for Frame {
    const LEVEL: usize = 0;
}

impl TableLevel for Pml4 {
    type Entry = PML4E;
    type NextLevel = Pdpt;
}

impl TopLevel for Pml4 {}

impl TableLevel for Pdpt {
    type Entry = PDPTE;
    type NextLevel = PageDirectory;
}

impl TableLevel for PageDirectory {
    type Entry = PDE;
    type NextLevel = Pt;
}

impl TableLevel for Pt {
    type Entry = PTE;
    type NextLevel = Frame;
}

impl PageLevel for Pdpt {
    const PAGE_BITS: usize = PAGE_BITS_1G;
}

impl PageLevel for PageDirectory {
    const PAGE_BITS: usize = PAGE_BITS_2M;
}

impl PageLevel for Pt {
    const PAGE_BITS: usize = PAGE_BITS_4K;
}
