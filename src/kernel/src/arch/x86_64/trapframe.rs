use core::arch::asm;

use x86_64::registers::rflags::RFlags;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TrapFrame {
    /// Registers in order: rax, rbx, rcx, rdx, rsi, rdi, rbp, r8-r15.
    pub registers: [usize; 15],

    // Error code (pushed bu CPU or by stub).
    pub error_code: usize,

    // Automatically saved by CPU.
    pub rip: usize,
    pub cs: usize,
    pub rflags: RFlags,
    pub rsp: usize,
    pub ss: usize,
}

impl TrapFrame {
    /// Create an empty [`TrapFrame`].
    pub const fn new() -> Self {
        unsafe { core::mem::zeroed() }
    }

    /// Restore userland context after context switching.
    pub unsafe fn restore(&mut self) -> ! {
        asm!(
            "mov rsp, {ptr}",
            "pop rax", "pop rbx", "pop rcx", "pop rdx",
            "pop rsi", "pop rdi", "pop rbp", "pop r8",
            "pop r9", "pop r10", "pop r11", "pop r12",
            "pop r13", "pop r14", "pop r15",
            "add rsp, 8",
            "iretq",
            ptr = in(reg) self,
            options(noreturn)
        );
    }

    /// Set message register.
    pub fn set_mr(&mut self, idx: usize, mr: usize) {
        self.registers[idx] = mr;
    }
}
