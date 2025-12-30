//! Physical frame capabilities for memory management.

use crate::error::{Result, SysError};
use crate::mask;
use crate::objects::capability::{CapRaw, CapRef, CapRights, ObjType};
use crate::objects::tcb::Tcb;
use crate::vspace::{CachePolicy, VMAttributes, VMRights};

/// Frame size variants supported by the architecture.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameSize {
    /// 4 KiB page (12 bits).
    #[default]
    Small = 12,
    /// 2 MiB large page (21 bits).
    Large = 21,
    /// 1 GiB huge page (30 bits).
    Huge = 30,
}

impl FrameSize {
    /// Returns the size in bytes for this frame type.
    #[inline]
    pub const fn bytes(self) -> usize {
        1 << (self as usize)
    }

    /// Returns the bit size (log2 of byte size).
    #[inline]
    pub const fn bits(self) -> usize {
        self as usize
    }

    /// Try to create a [`FrameSize`] from bit size.
    pub const fn from_bits(bits: usize) -> Option<Self> {
        match bits {
            12 => Some(Self::Small),
            21 => Some(Self::Large),
            30 => Some(Self::Huge),
            _ => None,
        }
    }

    /// Returns the alignment mask for this frame size.
    #[inline]
    pub const fn align_mask(self) -> usize {
        self.bytes() - 1
    }

    /// Check if an address is properly aligned for this frame size.
    #[inline]
    pub const fn is_aligned(self, addr: usize) -> bool {
        (addr & self.align_mask()) == 0
    }
}

#[derive(Debug)]
pub enum FrameObj {}

pub type FrameCap<'a> = CapRef<'a, FrameObj>;

impl FrameCap<'_> {
    const CACHE_POLICY_OFFSET: usize = 7;
    const CACHE_POLICY_WIDTH: usize = 3;
    const IS_DEVICE_OFFSET: usize = 6;
    const MAPPED_ASID_OFFSET: usize = 10;
    const MAPPED_ASID_WIDTH: usize = 16;
    const MAPPED_VADDR_OFFSET: usize = 0;
    const SIZE_BITS_OFFSET: usize = 0;
    const SIZE_BITS_WIDTH: usize = 6;

    /// Create a new frame capability.
    pub const fn mint(
        paddr: usize,
        size: FrameSize,
        is_device: bool,
        rights: CapRights,
    ) -> CapRaw {
        debug_assert!(
            (paddr & ((1 << size.bits()) - 1)) == 0,
            "Frame address must be aligned to frame size"
        );

        let cache = if is_device {
            CachePolicy::Uncacheable as usize
        } else {
            CachePolicy::WriteBack as usize
        };

        let arg1 = ((size as usize) << Self::SIZE_BITS_OFFSET) |
            ((is_device as usize) << Self::IS_DEVICE_OFFSET) |
            (cache << Self::CACHE_POLICY_OFFSET);

        let mut capraw = CapRaw::default_with_type(ObjType::Frame);
        capraw.paddr = paddr;
        capraw.arg1 = arg1;
        capraw.arg2 = 0; // Not mapped initially. 
        capraw.rights = rights;
        capraw
    }

    /// Create a frame capability with specific cache policy.
    pub const fn mint_with_cache(
        paddr: usize,
        size: FrameSize,
        is_device: bool,
        cache: CachePolicy,
        rights: CapRights,
    ) -> CapRaw {
        let arg1 = ((size as usize) << Self::SIZE_BITS_OFFSET) |
            ((is_device as usize) << Self::IS_DEVICE_OFFSET) |
            ((cache as usize) << Self::CACHE_POLICY_OFFSET);

        let mut capraw = CapRaw::default_with_type(ObjType::Frame);
        capraw.paddr = paddr;
        capraw.arg1 = arg1;
        capraw.arg2 = 0;
        capraw.rights = rights;
        capraw
    }

    /// Get the frame size.
    #[inline]
    pub fn size(&self) -> FrameSize {
        let raw = self.raw.get();
        let bits =
            (raw.arg1 >> Self::SIZE_BITS_OFFSET) & mask!(Self::SIZE_BITS_WIDTH);
        FrameSize::from_bits(bits).unwrap_or(FrameSize::Small)
    }

    /// Get the frame size in bytes.
    #[inline]
    pub fn size_bytes(&self) -> usize {
        self.size().bytes()
    }

    /// Get the frame size in bits (log2).
    #[inline]
    pub fn size_bits(&self) -> usize {
        self.size().bits()
    }

    /// Check if this is device memory.
    #[inline]
    pub fn is_device(&self) -> bool {
        let raw = self.raw.get();
        (raw.arg1 >> Self::IS_DEVICE_OFFSET) & 1 != 0
    }

    /// Get the cache policy for this frame.
    #[inline]
    pub fn cache_policy(&self) -> CachePolicy {
        let raw = self.raw.get();
        let policy = (raw.arg1 >> Self::CACHE_POLICY_OFFSET) &
            mask!(Self::CACHE_POLICY_WIDTH);
        match policy {
            0 => CachePolicy::WriteBack,
            1 => CachePolicy::WriteThrough,
            2 => CachePolicy::Uncacheable,
            3 => CachePolicy::WriteCombining,
            _ => CachePolicy::WriteBack,
        }
    }

    /// Set the cache policy.
    pub fn set_cache_policy(&self, policy: CachePolicy) {
        let mut raw = self.raw.get();
        raw.arg1 = (raw.arg1 &
            !(mask!(Self::CACHE_POLICY_WIDTH) << Self::CACHE_POLICY_OFFSET)) |
            ((policy as usize) << Self::CACHE_POLICY_OFFSET);
        self.raw.set(raw);
    }

    /// Get the mapped ASID (0 if unmapped).
    #[inline]
    pub fn mapped_asid(&self) -> u16 {
        let raw = self.raw.get();
        ((raw.arg1 >> Self::MAPPED_ASID_OFFSET) &
            mask!(Self::MAPPED_ASID_WIDTH)) as u16
    }

    /// Get the mapped virtual address (only valid if mapped).
    #[inline]
    pub fn mapped_vaddr(&self) -> usize {
        self.raw.get().arg2
    }

    /// Check if this frame is currently mapped.
    #[inline]
    pub fn is_mapped(&self) -> bool {
        self.mapped_asid() != 0
    }

    /// Record that this frame has been mapped.
    pub fn set_mapped(&self, asid: u16, vaddr: usize) -> Result<()> {
        if self.is_mapped() {
            return Err(SysError::FrameAlreadyMapped);
        }

        let mut raw = self.raw.get();
        raw.arg1 = (raw.arg1 &
            !((mask!(Self::MAPPED_ASID_WIDTH)) << Self::MAPPED_ASID_OFFSET)) |
            ((asid as usize) << Self::MAPPED_ASID_OFFSET);
        raw.arg2 = vaddr;
        self.raw.set(raw);
        Ok(())
    }

    /// Clear the mapping record.
    ///
    /// # Safety
    /// Caller must ensure the frame is actually unmapped from page tables
    /// before calling this.
    pub fn clear_mapped(&self) {
        let mut raw = self.raw.get();
        raw.arg1 &=
            !(mask!(Self::MAPPED_ASID_WIDTH) << Self::MAPPED_ASID_OFFSET);
        raw.arg2 = 0;
        self.raw.set(raw);
    }

    /// Convert capability rights to VM rights for mapping.
    pub fn vm_rights_from_cap(&self) -> VMRights {
        let cap_rights = self.rights();
        let mut vm_rights = VMRights::NONE;

        if cap_rights.contains(CapRights::READ) {
            vm_rights |= VMRights::READ;
        }
        if cap_rights.contains(CapRights::WRITE) {
            vm_rights |= VMRights::WRITE;
        }
        if cap_rights.contains(CapRights::EXECUTE) {
            vm_rights |= VMRights::EXECUTE;
        }

        vm_rights
    }

    /// Build VM attributes for mapping this frame.
    pub fn vm_attributes(&self, user: bool) -> VMAttributes {
        VMAttributes {
            rights: self.vm_rights_from_cap(),
            cache: self.cache_policy(),
            global: false,
            user,
        }
    }

    pub fn identify(&self, tcb: &mut Tcb) -> usize {
        tcb.set_mr(Tcb::MR1, self.cap_type() as usize);
        tcb.set_mr(Tcb::MR2, self.paddr().as_u64() as usize);
        tcb.set_mr(Tcb::MR3, self.size_bits());
        tcb.set_mr(Tcb::MR4, self.is_device() as usize);
        tcb.set_mr(Tcb::MR5, self.mapped_vaddr());
        tcb.set_mr(Tcb::MR6, self.mapped_asid() as usize);
        6
    }
}
