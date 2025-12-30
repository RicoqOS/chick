//! Translation Lookaside Buffer.

use core::arch::asm;

use x86_64::VirtAddr;

/// Flush a single page from the TLB.
#[inline]
pub fn flush_page(vaddr: VirtAddr) {
    unsafe {
        asm!(
            "invlpg [{0}]",
            in(reg) vaddr.as_u64(),
            options(nostack, preserves_flags)
        );
    }
}

/// Flush the entire TLB by reloading CR3.
#[inline]
pub fn flush_all() {
    unsafe {
        asm!(
            "mov {tmp}, cr3",
            "mov cr3, {tmp}",
            tmp = out(reg) _,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate all TLB entries for a specific PCID.
#[inline]
pub fn invalidate_pcid(pcid: u16) {
    #[repr(C)]
    struct InvpcidDesc {
        pcid: u64,
        addr: u64,
    }

    let desc = InvpcidDesc {
        pcid: pcid as u64,
        addr: 0,
    };

    unsafe {
        asm!(
            "invpcid {0}, [{1}]",
            in(reg) 0u64, // Type 0: Individual address
            in(reg) &desc,
            options(nostack, preserves_flags)
        );
    }
}

/// Invalidate all TLB entries except global pages.
#[inline]
pub fn flush_all_non_global() {
    // Toggle CR4.
    // PGE to flush non-global entries.
    unsafe {
        asm!(
            "mov {tmp}, cr4",
            "and {tmp}, ~(1 << 7)", // Clear PGE
            "mov cr4, {tmp}",
            "or {tmp}, (1 << 7)", // Set PGE
            "mov cr4, {tmp}",
            tmp = out(reg) _,
            options(nostack, preserves_flags)
        );
    }
}

/// Data Synchronization Barrier (memory fence).
#[inline]
pub fn dsb() {
    unsafe {
        asm!("mfence", options(nostack, preserves_flags));
    }
}

/// Instruction Synchronization Barrier.
#[inline]
pub fn isb() {
    unsafe {
        asm!(
            "push rax",
            "push rbx",
            "push rcx",
            "push rdx",
            "xor eax, eax",
            "cpuid",
            "pop rdx",
            "pop rcx",
            "pop rbx",
            "pop rax",
            options(nostack)
        );
    }
}
