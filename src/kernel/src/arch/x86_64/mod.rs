/// Advanced programmable interrupt controller
pub mod apic;

/// Advanced configuration and power interface.
pub mod acpi;

/// Console logger.
#[cfg(feature = "framebuffer")]
pub mod console;

/// x86 constants.
pub mod constants;

/// Interrupt descriptor table for CPU interrupts.
pub mod interrupts;

/// Programmable interrupt controller.
pub mod pic;

/// Programmable interval timer.
pub mod pit;

/// Handle PIT or LAPIC timer.
pub mod tick;

/// syscall, sysret handler.
pub mod syscall;

/// Trap frame.
pub mod trapframe;
pub mod vspace;

pub use x86_64::{PhysAddr, VirtAddr};

/// Halt CPU.
/// Disable interrupts if no task is scheduled or awaiting.
pub fn halt() {
    use x86_64::instructions::interrupts::enable_and_hlt;
    enable_and_hlt();
}

/// System data.
pub struct System {
    pub cores: u32,
}

/// Return system data.
pub fn sysinfo() -> System {
    let cores = {
        use core::arch::x86_64::__cpuid;
        let cpuid = __cpuid(4);
        ((cpuid.eax >> 26) & 0x3f) + 1
    };
    System { cores }
}

/// Return current core ID.
pub fn cpuid() -> u32 {
    let cpuid = core::arch::x86_64::__cpuid(1);
    (cpuid.ebx >> 24) & 0xff
}
