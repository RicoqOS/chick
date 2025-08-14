use core::ptr::NonNull;

use acpi::{AcpiHandler, PhysicalMapping};
use x86_64::{PhysAddr, VirtAddr};

/// ACPI handler.
#[derive(Debug, Clone, Copy)]
pub struct Acpi {
    pub physical_memory_offset: VirtAddr,
}

impl Acpi {
    /// Create a new [`Acpi`] handler.
    pub fn new(physical_memory_offset: VirtAddr) -> Self {
        Self {
            physical_memory_offset,
        }
    }
}

unsafe impl Send for Acpi {}
unsafe impl Sync for Acpi {}

impl AcpiHandler for Acpi {
    /// Map physical memory region to virtual memory.
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let phys_addr = PhysAddr::new(physical_address as u64);
        let virt_addr = self.physical_memory_offset + phys_addr.as_u64();

        unsafe {
            PhysicalMapping::new(
                physical_address,
                NonNull::new(virt_addr.as_mut_ptr()).expect("failed to get virtual address"),
                size,
                size,
                *self,
            )
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {}
}
