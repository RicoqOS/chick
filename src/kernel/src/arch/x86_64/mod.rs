/// Advanced programmable interrupt controller
pub mod apic;

/// Advanced configuration and power interface.
pub mod acpi;

/// Console logger.
pub mod console;

/// Interrupt descriptor table for CPU interrupts.
pub mod interrupts;

/// Virtual memory.
/// Fixed-size with linked list fallback allocator.
pub mod mm;

/// Programmable interrupt controller.
pub mod pic;

/// Programmable interval timer.
pub mod pit;
