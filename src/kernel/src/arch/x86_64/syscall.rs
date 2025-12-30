use core::arch::naked_asm;

use x86_64::VirtAddr;
use x86_64::registers::model_specific::{Efer, EferFlags, LStar, SFMask, Star};
use x86_64::registers::rflags::RFlags;

use crate::arch::interrupts::gdt::GDT;

/// Set method handler for syscalls.
pub fn init_syscall() {
    let user_cs = GDT.1.user_code_selector.0;
    let kernel_cs = GDT.1.code_selector.0;

    log::debug!("selectors are {user_cs} (user) and {kernel_cs} (kernel)");

    unsafe { Star::write_raw(user_cs, kernel_cs) }

    let addr = syscall_stub as *const ();
    let handler = VirtAddr::new(addr as u64);
    LStar::write(handler);

    let flags = RFlags::from_bits((1 << 9) | (1 << 10)).unwrap();
    SFMask::write(flags);

    let mut efer = Efer::read();
    efer.insert(EferFlags::SYSTEM_CALL_EXTENSIONS);
    efer.insert(EferFlags::LONG_MODE_ENABLE);
    efer.insert(EferFlags::LONG_MODE_ACTIVE);
    unsafe { Efer::write(efer) };
}

/// Syscall registers.
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct Regs {
    /// Syscall number.
    pub rax: u64,
    // Destination index.
    pub rdi: u64,
    // Source index.
    pub rsi: u64,
    /// Data register.
    pub rdx: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    /// RFLAGS (CPU flags).
    pub r11: u64,
    pub rcx: u64,
}

#[unsafe(naked)]
extern "C" fn syscall_stub() {
    naked_asm!(
        "swapgs", // switch GS -> kernel
        // Save fallbakcs.
        "push rcx", // user RIP.
        "push r11", // user RFLAGS.
        // Build register on stack.
        "push r10",
        "push r9",
        "push r8",
        "push rdx",
        "push rsi",
        "push rdi",
        "push rax",
        "mov rdi, rsp",
        "call syscall_entry",
        // Restore stack.
        "pop rax",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop r8",
        "pop r9",
        "pop r10",
        "pop r11",
        "pop rcx",
        "swapgs",
        "sysretq",
    )
}

#[unsafe(no_mangle)]
extern "C" fn syscall_entry(registers: &mut Regs) -> u64 {
    let args = [
        registers.rdi,
        registers.rsi,
        registers.rdx,
        registers.r10,
        registers.r8,
        registers.r9,
    ];
    let _ret = crate::syscall::handler(
        registers.rax,
        args
    );

    1
}
