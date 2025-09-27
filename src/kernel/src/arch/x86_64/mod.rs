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

/// Virtual memory.
/// Fixed-size with linked list fallback allocator.
pub mod mm;

/// Programmable interrupt controller.
pub mod pic;

/// Programmable interval timer.
pub mod pit;

/// Handle PIT or LAPIC timer.
pub mod tick;

/// syscall, sysret handler.
pub mod syscall;

/// Halt CPU.
/// Disable interrupts if no task is scheduled or awaiting.
pub fn halt(is_task_queue_empty: bool) {
    use x86_64::instructions::interrupts::{self, enable_and_hlt};

    interrupts::disable();
    if is_task_queue_empty {
        enable_and_hlt();
    } else {
        interrupts::enable();
    }
}

/// System data.
pub struct System {
    pub cores: u32,
}

/// Return system data.
pub fn sysinfo() -> System {
    let cores = {
        use core::arch::x86_64::__cpuid;
        let cpuid = unsafe { __cpuid(4) };
        ((cpuid.eax >> 26) & 0x3f) + 1
    };
    System { cores }
}

/// Return current core ID.
pub fn cpuid() -> u32 {
    let cpuid = unsafe { core::arch::x86_64::__cpuid(1) };
    (cpuid.ebx >> 24) & 0xff
}
