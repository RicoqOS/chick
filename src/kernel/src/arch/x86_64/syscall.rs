use x86_64::registers::model_specific::{Msr, Star, LStar, SFMask};
use x86_64::registers::rflags::RFlags;
use x86_64::VirtAddr;

/// Set method handler for syscalls.
/// Only work on long mode.
pub fn init_syscall(handler: VirtAddr) {
    unsafe { Star::write_raw(0x23, 0x10) }
    
    unsafe { LStar::write(handler); }

    let flags = RFlags::from_bits(1 << 9 | 1 << 10).unwrap();
    unsafe { SFMask::write(flags); }
}
