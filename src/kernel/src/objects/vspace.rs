//! Virtual address space capabilities.

use crate::arch::vspace::tlb::flush_page;
#[cfg(target_arch = "x86_64")]
use crate::arch::vspace::{
    entry::{PageTableEntry, Pde, Pdpte, Pml4e, Pte},
    level::{PageDirectory, Pdpt, Pml4, Pt},
};
use crate::arch::{PhysAddr, VirtAddr};
use crate::error::{VSpaceError, WalkResult};
use crate::mask;
use crate::objects::frame::{FrameCap, FrameSize};
use crate::objects::tcb::Tcb;
use crate::objects::{CapRaw, CapRef, CapRights, ObjType};
use crate::vspace::{Table, VMAttributes};

pub type Asid = u16;

/// Maximum number of ASIDs supported.
pub const ASID_MAX: Asid = 0xFFFF;

#[derive(Debug)]
pub enum VSpaceObj {}

pub type VSpaceCap<'a> = CapRef<'a, VSpaceObj>;

impl VSpaceCap<'_> {
    const ASID_OFFSET: usize = 0;
    const ASID_WIDTH: usize = 16;
    const IS_ACTIVE_OFFSET: usize = 16;

    pub const fn mint(
        pml4_paddr: usize,
        asid: Asid,
        rights: CapRights,
    ) -> CapRaw {
        debug_assert!(
            (pml4_paddr & 0xFFF) == 0,
            "PML4 address must be 4K aligned"
        );

        let arg1 = (asid as usize) << Self::ASID_OFFSET;

        let mut capraw = CapRaw::default_with_type(ObjType::VSpace);
        capraw.paddr = pml4_paddr;
        capraw.arg1 = arg1;
        capraw.arg2 = 0;
        capraw.rights = rights;
        capraw
    }

    #[inline]
    pub fn asid(&self) -> Asid {
        let raw = self.raw.get();
        ((raw.arg1 >> Self::ASID_OFFSET) & mask!(Self::ASID_WIDTH)) as Asid
    }

    #[inline]
    pub fn is_active(&self) -> bool {
        let raw = self.raw.get();
        (raw.arg1 >> Self::IS_ACTIVE_OFFSET) & 1 != 0
    }

    pub fn set_active(&self, active: bool) {
        let mut raw = self.raw.get();
        if active {
            raw.arg1 |= 1 << Self::IS_ACTIVE_OFFSET;
        } else {
            raw.arg1 &= !(1 << Self::IS_ACTIVE_OFFSET);
        }
        self.raw.set(raw);
    }

    #[inline]
    pub fn root_paddr(&self) -> PhysAddr {
        self.paddr()
    }

    pub fn identify(&self, tcb: &mut Tcb) -> usize {
        tcb.set_mr(Tcb::MR1, self.cap_type() as usize);
        tcb.set_mr(Tcb::MR2, self.paddr().as_u64() as usize);
        tcb.set_mr(Tcb::MR3, self.asid() as usize);
        tcb.set_mr(Tcb::MR4, self.is_active() as usize);
        4
    }
}

#[cfg(target_arch = "x86_64")]
impl VSpaceCap<'_> {
    #[inline]
    pub const fn vaddr_indices(vaddr: usize) -> (usize, usize, usize, usize) {
        let pml4_idx = (vaddr >> 39) & 0x1FF;
        let pdpt_idx = (vaddr >> 30) & 0x1FF;
        let pd_idx = (vaddr >> 21) & 0x1FF;
        let pt_idx = (vaddr >> 12) & 0x1FF;
        (pml4_idx, pdpt_idx, pd_idx, pt_idx)
    }

    #[inline]
    pub const fn is_canonical(vaddr: usize) -> bool {
        let top_bits = vaddr >> 47;
        top_bits == 0 || top_bits == 0x1FFFF
    }

    #[inline]
    pub unsafe fn pml4<const OFFSET: u64>(&self) -> &'static mut Table<Pml4> {
        Table::<Pml4>::from_paddr::<OFFSET>(self.root_paddr())
    }

    pub unsafe fn walk<const OFFSET: u64>(
        &self,
        vaddr: VirtAddr,
    ) -> Result<WalkResult, VSpaceError> {
        if !Self::is_canonical(vaddr.as_u64() as usize) {
            return Err(VSpaceError::InvalidVAddr);
        }

        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) =
            Self::vaddr_indices(vaddr.as_u64() as usize);

        let pml4 = self.pml4::<OFFSET>();
        let pml4e = &pml4[pml4_idx];

        if !pml4e.is_present() {
            return Ok(WalkResult::NotMapped { level: 4 });
        }

        let pdpt: &mut Table<Pdpt> =
            Table::from_paddr::<OFFSET>(pml4e.paddr());
        let pdpte = &pdpt[pdpt_idx];

        if !pdpte.is_present() {
            return Ok(WalkResult::NotMapped { level: 3 });
        }

        if pdpte.is_page() {
            return Ok(WalkResult::MappedPage {
                paddr: pdpte.paddr().as_u64() as usize,
                size: FrameSize::Huge,
                level: 3,
            });
        }

        let pd: &mut Table<PageDirectory> =
            Table::from_paddr::<OFFSET>(pdpte.paddr());
        let pde = &pd[pd_idx];

        if !pde.is_present() {
            return Ok(WalkResult::NotMapped { level: 2 });
        }

        if pde.is_page() {
            return Ok(WalkResult::MappedPage {
                paddr: pde.paddr().as_u64() as usize,
                size: FrameSize::Large,
                level: 2,
            });
        }

        let pt: &mut Table<Pt> = Table::from_paddr::<OFFSET>(pde.paddr());
        let pte = &pt[pt_idx];

        if !pte.is_present() {
            return Ok(WalkResult::NotMapped { level: 1 });
        }

        Ok(WalkResult::MappedPage {
            paddr: pte.paddr().as_u64() as usize,
            size: FrameSize::Small,
            level: 1,
        })
    }

    pub unsafe fn map_4k<const OFFSET: u64>(
        &self,
        vaddr: VirtAddr,
        frame_paddr: PhysAddr,
        attr: VMAttributes,
    ) -> Result<(), VSpaceError> {
        if !Self::is_canonical(vaddr.as_u64() as usize) {
            return Err(VSpaceError::InvalidVAddr);
        }

        if !FrameSize::Small.is_aligned(vaddr.as_u64() as usize) {
            return Err(VSpaceError::MisalignedVAddr);
        }

        if !FrameSize::Small.is_aligned(frame_paddr.as_u64() as usize) {
            return Err(VSpaceError::MisalignedPAddr);
        }

        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) =
            Self::vaddr_indices(vaddr.as_u64() as usize);

        let pml4 = self.pml4::<OFFSET>();
        let pml4e = &pml4[pml4_idx];

        if !pml4e.is_present() {
            return Err(VSpaceError::MissingTable);
        }

        let pdpt: &mut Table<Pdpt> =
            Table::from_paddr::<OFFSET>(pml4e.paddr());
        let pdpte = &pdpt[pdpt_idx];

        if !pdpte.is_present() {
            return Err(VSpaceError::MissingTable);
        }

        if pdpte.is_page() {
            return Err(VSpaceError::AlreadyMapped);
        }

        let pd: &mut Table<PageDirectory> =
            Table::from_paddr::<OFFSET>(pdpte.paddr());
        let pde = &pd[pd_idx];

        if !pde.is_present() {
            return Err(VSpaceError::MissingTable);
        }

        if pde.is_page() {
            return Err(VSpaceError::AlreadyMapped);
        }

        let pt: &mut Table<Pt> = Table::from_paddr::<OFFSET>(pde.paddr());
        let pte = &mut pt[pt_idx];

        if pte.is_present() {
            return Err(VSpaceError::AlreadyMapped);
        }

        *pte = Pte::new_page(frame_paddr, attr);
        flush_page(vaddr);

        Ok(())
    }

    pub unsafe fn map_2m<const OFFSET: u64>(
        &self,
        vaddr: VirtAddr,
        frame_paddr: PhysAddr,
        attr: VMAttributes,
    ) -> Result<(), VSpaceError> {
        if !Self::is_canonical(vaddr.as_u64() as usize) {
            return Err(VSpaceError::InvalidVAddr);
        }

        if !FrameSize::Large.is_aligned(vaddr.as_u64() as usize) {
            return Err(VSpaceError::MisalignedVAddr);
        }

        if !FrameSize::Large.is_aligned(frame_paddr.as_u64() as usize) {
            return Err(VSpaceError::MisalignedPAddr);
        }

        let (pml4_idx, pdpt_idx, pd_idx, _) =
            Self::vaddr_indices(vaddr.as_u64() as usize);

        let pml4 = self.pml4::<OFFSET>();
        let pml4e = &pml4[pml4_idx];

        if !pml4e.is_present() {
            return Err(VSpaceError::MissingTable);
        }

        let pdpt: &mut Table<Pdpt> =
            Table::from_paddr::<OFFSET>(pml4e.paddr());
        let pdpte = &pdpt[pdpt_idx];

        if !pdpte.is_present() {
            return Err(VSpaceError::MissingTable);
        }

        if pdpte.is_page() {
            return Err(VSpaceError::AlreadyMapped);
        }

        let pd: &mut Table<PageDirectory> =
            Table::from_paddr::<OFFSET>(pdpte.paddr());
        let pde = &mut pd[pd_idx];

        if pde.is_present() {
            return Err(VSpaceError::AlreadyMapped);
        }

        *pde = Pde::new_large_page(frame_paddr, attr);
        flush_page(vaddr);

        Ok(())
    }

    pub unsafe fn map_1g<const OFFSET: u64>(
        &self,
        vaddr: VirtAddr,
        frame_paddr: PhysAddr,
        attr: VMAttributes,
    ) -> Result<(), VSpaceError> {
        if !Self::is_canonical(vaddr.as_u64() as usize) {
            return Err(VSpaceError::InvalidVAddr);
        }

        if !FrameSize::Huge.is_aligned(vaddr.as_u64() as usize) {
            return Err(VSpaceError::MisalignedVAddr);
        }

        if !FrameSize::Huge.is_aligned(frame_paddr.as_u64() as usize) {
            return Err(VSpaceError::MisalignedPAddr);
        }

        let (pml4_idx, pdpt_idx, _, _) =
            Self::vaddr_indices(vaddr.as_u64() as usize);

        let pml4 = self.pml4::<OFFSET>();
        let pml4e = &pml4[pml4_idx];

        if !pml4e.is_present() {
            return Err(VSpaceError::MissingTable);
        }

        let pdpt: &mut Table<Pdpt> =
            Table::from_paddr::<OFFSET>(pml4e.paddr());
        let pdpte = &mut pdpt[pdpt_idx];

        if pdpte.is_present() {
            return Err(VSpaceError::AlreadyMapped);
        }

        *pdpte = Pdpte::new_huge_page(frame_paddr, attr);
        flush_page(vaddr);

        Ok(())
    }

    pub unsafe fn map_frame<const OFFSET: u64>(
        &self,
        vaddr: VirtAddr,
        frame: &FrameCap<'_>,
        user: bool,
    ) -> Result<(), VSpaceError> {
        let frame_paddr = frame.paddr();
        let attr = frame.vm_attributes(user);

        match frame.size() {
            FrameSize::Small => {
                self.map_4k::<OFFSET>(vaddr, frame_paddr, attr)
            },
            FrameSize::Large => {
                self.map_2m::<OFFSET>(vaddr, frame_paddr, attr)
            },
            FrameSize::Huge => self.map_1g::<OFFSET>(vaddr, frame_paddr, attr),
        }
    }

    pub unsafe fn unmap<const OFFSET: u64>(
        &self,
        vaddr: VirtAddr,
    ) -> Result<(PhysAddr, FrameSize), VSpaceError> {
        if !Self::is_canonical(vaddr.as_u64() as usize) {
            return Err(VSpaceError::InvalidVAddr);
        }

        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) =
            Self::vaddr_indices(vaddr.as_u64() as usize);

        let pml4 = self.pml4::<OFFSET>();
        let pml4e = &pml4[pml4_idx];

        if !pml4e.is_present() {
            return Err(VSpaceError::NotMapped);
        }

        let pdpt: &mut Table<Pdpt> =
            Table::from_paddr::<OFFSET>(pml4e.paddr());
        let pdpte = &mut pdpt[pdpt_idx];

        if !pdpte.is_present() {
            return Err(VSpaceError::NotMapped);
        }

        if pdpte.is_page() {
            let paddr = pdpte.paddr();
            *pdpte = Pdpte::invalid();
            flush_page(vaddr);
            return Ok((paddr, FrameSize::Huge));
        }

        let pd: &mut Table<PageDirectory> =
            Table::from_paddr::<OFFSET>(pdpte.paddr());
        let pde = &mut pd[pd_idx];

        if !pde.is_present() {
            return Err(VSpaceError::NotMapped);
        }

        if pde.is_page() {
            let paddr = pde.paddr();
            *pde = Pde::invalid();
            flush_page(vaddr);
            return Ok((paddr, FrameSize::Large));
        }

        let pt: &mut Table<Pt> = Table::from_paddr::<OFFSET>(pde.paddr());
        let pte = &mut pt[pt_idx];

        if !pte.is_present() {
            return Err(VSpaceError::NotMapped);
        }

        let paddr = pte.paddr();
        *pte = Pte::invalid();
        flush_page(vaddr);

        Ok((paddr, FrameSize::Small))
    }

    pub unsafe fn install_table<const OFFSET: u64>(
        &self,
        vaddr: VirtAddr,
        level: usize,
        table_paddr: PhysAddr,
    ) -> Result<(), VSpaceError> {
        if !Self::is_canonical(vaddr.as_u64() as usize) {
            return Err(VSpaceError::InvalidVAddr);
        }

        if !FrameSize::Small.is_aligned(table_paddr.as_u64() as usize) {
            return Err(VSpaceError::MisalignedPAddr);
        }

        let (pml4_idx, pdpt_idx, pd_idx, _) =
            Self::vaddr_indices(vaddr.as_u64() as usize);

        match level {
            3 => {
                let pml4 = self.pml4::<OFFSET>();
                let pml4e = &mut pml4[pml4_idx];

                if pml4e.is_present() {
                    return Err(VSpaceError::AlreadyMapped);
                }

                *pml4e = Pml4e::new_table(table_paddr);
            },
            2 => {
                let pml4 = self.pml4::<OFFSET>();
                let pml4e = &pml4[pml4_idx];

                if !pml4e.is_present() {
                    return Err(VSpaceError::MissingTable);
                }

                let pdpt: &mut Table<Pdpt> =
                    Table::from_paddr::<OFFSET>(pml4e.paddr());
                let pdpte = &mut pdpt[pdpt_idx];

                if pdpte.is_present() {
                    return Err(VSpaceError::AlreadyMapped);
                }

                *pdpte = Pdpte::new_table(table_paddr);
            },
            1 => {
                let pml4 = self.pml4::<OFFSET>();
                let pml4e = &pml4[pml4_idx];

                if !pml4e.is_present() {
                    return Err(VSpaceError::MissingTable);
                }

                let pdpt: &mut Table<Pdpt> =
                    Table::from_paddr::<OFFSET>(pml4e.paddr());
                let pdpte = &pdpt[pdpt_idx];

                if !pdpte.is_present() {
                    return Err(VSpaceError::MissingTable);
                }

                if pdpte.is_page() {
                    return Err(VSpaceError::AlreadyMapped);
                }

                let pd: &mut Table<PageDirectory> =
                    Table::from_paddr::<OFFSET>(pdpte.paddr());
                let pde = &mut pd[pd_idx];

                if pde.is_present() {
                    return Err(VSpaceError::AlreadyMapped);
                }

                *pde = Pde::new_table(table_paddr);
            },
            _ => return Err(VSpaceError::InvalidVAddr),
        }

        Ok(())
    }
}
