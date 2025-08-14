use x86_64::{
    PhysAddr, VirtAddr,
    structures::paging::{FrameAllocator, Mapper, PhysFrame, Size4KiB},
};

use core::arch::x86_64::__cpuid;

use crate::arch::pic;

fn has_apic() -> bool {
    let xd_bit: u32 = 1 << 9;

    let edx = unsafe {
        // Execute the CPUID instruction with EAX=1 and ECX=0.
        let result = __cpuid(1);

        result.edx
    };

    // Check if the XD bit is set in the EDX register.
    (edx & xd_bit) != 0
}

fn enable_io_apic(addr: VirtAddr) {
    let ptr = addr.as_mut_ptr::<u32>();
    unsafe { ptr.offset(0).write_volatile(0x12) };
}

fn map_apic(
    physical_address: u64,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> VirtAddr {
    use x86_64::structures::paging::{Page, PageTableFlags as Flags};

    let physical_address = PhysAddr::new(physical_address);
    let page = Page::containing_address(VirtAddr::new(physical_address.as_u64()));
    let frame = PhysFrame::containing_address(physical_address);

    let flags = Flags::PRESENT | Flags::WRITABLE | Flags::NO_CACHE;

    unsafe {
        mapper
            .map_to(page, frame, flags, frame_allocator)
            .expect("APIC mapping failed")
            .flush();
    }

    page.start_address()
}

/// APIC manager.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct Apic {
    io_apic_addr: VirtAddr,
    lapic_addr: VirtAddr,
}

impl Apic {
    /// Create a new [`Apic`], with LAPIC memory.
    pub fn new(
        rsdp_addr: usize,
        physical_memory_offset: VirtAddr,
        mapper: &mut impl Mapper<Size4KiB>,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Self {
        if !has_apic() {
            panic!("apic is not supported (1)");
        }

        pic::Pic::new().disable();

        let acpi = crate::arch::acpi::Acpi::new(physical_memory_offset);
        let acpi_tables = unsafe {
            acpi::AcpiTables::from_rsdp(acpi, rsdp_addr).expect("Failed to parse ACPI tables")
        };

        let platform_info = acpi_tables
            .platform_info()
            .expect("Failed to get platform info");

        let (io_apic_addr, lapic_addr) = match platform_info.interrupt_model {
            acpi::InterruptModel::Apic(apic) => {
                let apic_addr = apic.io_apics[0].address;
                log::debug!("apic address is {apic_addr:?}");

                let lapic_addr = apic.local_apic_address;
                log::debug!("lapic address is {lapic_addr:?}");

                (apic_addr as u64, lapic_addr)
            }
            _ => panic!("apic is not supported (2)"),
        };

        let io_apic_addr = map_apic(io_apic_addr, mapper, frame_allocator);
        enable_io_apic(io_apic_addr);

        let lapic_addr = map_apic(lapic_addr, mapper, frame_allocator);

        log::info!("apic, lapic initialized");

        Self {
            io_apic_addr,
            lapic_addr,
        }
    }
}
