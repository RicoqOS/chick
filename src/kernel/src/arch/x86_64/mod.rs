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
