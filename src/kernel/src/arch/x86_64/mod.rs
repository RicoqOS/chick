///
pub mod apic;

/// Console logger.
pub mod console;

/// Interrupt descriptor table for CPU interrupts.
pub mod interrupts;

/// Virtual memory.
/// Fixed-size with linked list fallback allocator.
pub mod mm;
