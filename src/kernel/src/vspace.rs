use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

use bitflags::bitflags;

use crate::arch::vspace::entry::PageTableEntry;
use crate::arch::{PhysAddr, VirtAddr};

pub const PAGE_SIZE_4K: usize = 4096;
pub const PAGE_SIZE_2M: usize = 2 * 1024 * 1024;
pub const PAGE_SIZE_1G: usize = 1024 * 1024 * 1024;
pub const PAGE_BITS_4K: usize = 12;
pub const PAGE_BITS_2M: usize = 21;
pub const PAGE_BITS_1G: usize = 30;
pub const ENTRIES_PER_TABLE: usize = 512;
pub const ENTRIES_BITS: usize = 9;

pub trait Level: Copy + Clone {
    const LEVEL: usize;
}

pub trait TableLevel: Level {
    type Entry: Copy + Clone;
    type NextLevel: Level;
    const TABLE_ENTRIES: usize = ENTRIES_PER_TABLE;
}

pub trait PageLevel: Level {
    const PAGE_BITS: usize;
    const PAGE_SIZE: usize = 1 << Self::PAGE_BITS;
}

/// Marker for the top-level table.
pub trait TopLevel: TableLevel {}

bitflags! {
    /// Virtual memory access rights.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VMRights: u8 {
        const NONE      = 0b0000;
        const READ      = 0b0001;
        const WRITE     = 0b0010;
        const EXECUTE   = 0b0100;
        const KERNEL    = 0b1000;

        const RW    = Self::READ.bits() | Self::WRITE.bits();
        const RX    = Self::READ. bits() | Self::EXECUTE.bits();
        const RWX   = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE. bits();
    }
}

/// Cache policy for memory mappings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum CachePolicy {
    #[default]
    WriteBack = 0,
    WriteThrough = 1,
    Uncacheable = 2,
    WriteCombining = 3,
}

/// VM attributes for a mapping.
#[derive(Debug, Clone, Copy)]
pub struct VMAttributes {
    pub rights: VMRights,
    pub cache: CachePolicy,
    /// Global page (not flushed on CR3 switch).
    pub global: bool,
    /// User accessible.
    pub user: bool,
}

impl Default for VMAttributes {
    fn default() -> Self {
        Self {
            rights: VMRights::RW,
            cache: CachePolicy::WriteBack,
            global: false,
            user: true,
        }
    }
}

impl VMAttributes {
    /// Create kernel-only attributes.
    pub const fn kernel(rights: VMRights) -> Self {
        Self {
            rights,
            cache: CachePolicy::WriteBack,
            global: true,
            user: false,
        }
    }

    /// Create user attributes.
    pub const fn user(rights: VMRights) -> Self {
        Self {
            rights,
            cache: CachePolicy::WriteBack,
            global: false,
            user: true,
        }
    }

    /// Create device (uncacheable) attributes.
    pub const fn device() -> Self {
        Self {
            rights: VMRights::RW,
            cache: CachePolicy::Uncacheable,
            global: false,
            user: false,
        }
    }
}

/// A page table at a specific level.
#[repr(C, align(4096))]
pub struct Table<L: TableLevel> {
    entries: [L::Entry; ENTRIES_PER_TABLE],
    _marker: PhantomData<L>,
}

impl<L: TableLevel> Table<L>
where
    L::Entry: PageTableEntry,
{
    /// Create an empty [`Table`] with all invalid entries.
    pub fn new() -> Self
    where
        L::Entry: Default,
    {
        Self {
            entries: [L::Entry::default(); ENTRIES_PER_TABLE],
            _marker: PhantomData,
        }
    }

    /// Create a [`Table`] from a physical address.
    ///
    /// # Safety
    /// Caller must ensure `paddr` points to a valid, aligned page table.
    pub unsafe fn from_paddr<const OFFSET: u64>(
        paddr: PhysAddr,
    ) -> &'static mut Self {
        let vaddr = paddr.as_u64() + OFFSET;
        &mut *(vaddr as *mut Self)
    }

    /// Create a [`Table`] from a virtual address.
    ///
    /// # Safety
    /// The caller must ensure `vaddr` points to a valid, aligned page table.
    pub unsafe fn from_vaddr(vaddr: VirtAddr) -> &'static mut Self {
        &mut *(vaddr.as_u64() as *mut Self)
    }

    /// Get the physical address of this table.
    pub fn paddr<const OFFSET: u64>(&self) -> PhysAddr {
        let vaddr = self as *const _ as u64;
        PhysAddr::new(vaddr - OFFSET)
    }

    /// Get a reference to an entry by index.
    pub fn get(&self, index: usize) -> Option<&L::Entry> {
        self.entries.get(index)
    }

    /// Get a mutable reference to an entry by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut L::Entry> {
        self.entries.get_mut(index)
    }

    /// Iterate over all entries.
    pub fn iter(&self) -> impl Iterator<Item = &L::Entry> {
        self.entries.iter()
    }

    /// Iterate mutably over all entries.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut L::Entry> {
        self.entries.iter_mut()
    }

    /// Zero all entries.
    pub fn clear(&mut self)
    where
        L::Entry: PageTableEntry,
    {
        for entry in &mut self.entries {
            *entry = L::Entry::invalid();
        }
    }

    /// Get the next-level table from an entry.
    ///
    /// # Safety
    /// The caller must ensure the entry is a valid table entry.
    pub unsafe fn next_table<const OFFSET: u64>(
        &self,
        index: usize,
    ) -> Option<&'static mut Table<L::NextLevel>>
    where
        L::NextLevel: TableLevel,
        L::Entry: PageTableEntry,
        <L::NextLevel as TableLevel>::Entry: PageTableEntry,
    {
        let entry = self.get(index)?;
        if !entry.is_table() {
            return None;
        }
        let paddr = entry.paddr();
        Some(Table::<L::NextLevel>::from_paddr::<OFFSET>(paddr))
    }
}

impl<L: TableLevel> Index<usize> for Table<L> {
    type Output = L::Entry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L: TableLevel> IndexMut<usize> for Table<L> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl<L: TableLevel> Default for Table<L>
where
    L::Entry: PageTableEntry + Default,
{
    fn default() -> Self {
        Self::new()
    }
}
