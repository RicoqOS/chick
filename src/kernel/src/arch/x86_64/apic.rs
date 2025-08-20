use core::arch::x86_64::__cpuid;

use x86_64::structures::paging::{FrameAllocator, Mapper, PhysFrame, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

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

fn enable_lapic(addr: VirtAddr) {
    let ptr = addr.as_mut_ptr::<u32>();
    unsafe {
        let svr = ptr.offset(0xF0 / 4);
        svr.write_volatile(svr.read_volatile() | 0x100);
    };
}

fn map_apic(
    physical_address: u64,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> VirtAddr {
    use x86_64::structures::paging::{Page, PageTableFlags as Flags};

    let physical_address = PhysAddr::new(physical_address);
    let page =
        Page::containing_address(VirtAddr::new(physical_address.as_u64()));
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
    /// Create a new [`Apic`].
    pub fn new() -> Self {
        Self {
            io_apic_addr: VirtAddr::zero(),
            lapic_addr: VirtAddr::zero(),
        }
    }

    /// Inits MMIO.
    pub fn init(
        mut self,
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
            acpi::AcpiTables::from_rsdp(acpi, rsdp_addr)
                .expect("Failed to parse ACPI tables")
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

                log::debug!(
                    "apic interrupt source overrides: {:?}",
                    apic.interrupt_source_overrides
                );

                (apic_addr as u64, lapic_addr)
            },
            _ => panic!("apic is not supported (2)"),
        };

        let io_apic_addr = map_apic(io_apic_addr, mapper, frame_allocator);
        enable_io_apic(io_apic_addr);

        let lapic_addr = map_apic(lapic_addr, mapper, frame_allocator);
        enable_lapic(lapic_addr);

        log::info!("apic, lapic initialized");

        self.io_apic_addr = io_apic_addr;
        self.lapic_addr = lapic_addr;

        self
    }

    pub fn ioapic_read(&self, reg: u32) -> u32 {
        let base = self.io_apic_addr.as_mut_ptr::<u32>();

        unsafe {
            core::ptr::write_volatile(base, reg);
            core::ptr::read_volatile(base.add(4))
        }
    }

    pub fn ioapic_write(&self, reg: u32, value: u32) {
        let base = self.io_apic_addr.as_mut_ptr::<u32>();

        unsafe {
            core::ptr::write_volatile(base, reg);
            core::ptr::write_volatile(base.add(4), value);
        }
    }

    pub fn init_counter(&self, periodic: bool, ticks: u32) -> u32 {
        let ptr = self.lapic_addr.as_mut_ptr::<u32>();

        unsafe {
            let lvtt = ptr.offset(0x320 / 4);
            lvtt.write_volatile(0x20 | ((periodic as u32) << 17));

            let tdcr = ptr.offset(0x3E0 / 4);
            tdcr.write_volatile(0x1);

            let ticr = ptr.offset(0x380 / 4);
            ticr.write_volatile(ticks);
        };

        ticks
    }

    pub fn read_counter(&self) -> u32 {
        let ptr = self.lapic_addr.as_mut_ptr::<u32>();
        unsafe { ptr.add(0x390 / 4).read_volatile() }
    }

    pub fn end_interrupt(&self) {
        unsafe {
            let ptr = self.lapic_addr.as_mut_ptr::<u32>();
            ptr.offset(0xB0 / 4).write_volatile(0);
        }
    }
}

impl Default for Apic {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Sync for Apic {}
unsafe impl Send for Apic {}
