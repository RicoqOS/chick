use core::arch::x86_64::__cpuid;

use crate::arch::constants::apic::*;
use crate::arch::{VirtAddr, pic};
#[derive(Debug, Clone, Copy)]
pub struct Apic {
    io_apic_addr: VirtAddr,
    lapic_addr: VirtAddr,
}

impl Apic {
    /// Create an [`Apic`] with no address.
    pub const fn new() -> Self {
        Self {
            io_apic_addr: VirtAddr::zero(),
            lapic_addr: VirtAddr::zero(),
        }
    }

    fn has_apic() -> bool {
        let apic_bit = 1 << 9;
        let cpuid_result = unsafe { __cpuid(1) };
        (cpuid_result.edx & apic_bit) != 0
    }

    fn enable_io_apic(addr: VirtAddr) {
        let ptr = addr.as_mut_ptr::<u32>();
        unsafe { ptr.offset(0).write_volatile(0x12) };
    }

    fn enable_lapic(addr: VirtAddr) {
        let ptr = addr.as_mut_ptr::<u32>();
        unsafe {
            let svr = ptr.offset(ApicRegister::LapicSivr as isize / 4);
            svr.write_volatile(
                svr.read_volatile() | ApicValue::SvrEnable as u32,
            );
        };
    }

    /// Map an MMIO page.
    fn map_apic(paddr: u64, vspace_offset: u64) -> VirtAddr {
        // TODO: Use vspace.
        VirtAddr::new(paddr + vspace_offset)
    }

    /// APIC initialization.
    pub fn init(mut self, _rsdp_addr: usize, vspace_offset: u64) -> Self {
        if !Self::has_apic() {
            panic!("APIC is not supported");
        }

        pic::Pic::new().disable();

        // TODO: Read RDSP without alloc.
        let io_apic_addr = 0xFEC0_0000;
        let lapic_addr = 0xFEE0_0000;

        let io_apic_addr = Self::map_apic(io_apic_addr, vspace_offset);
        let lapic_addr = Self::map_apic(lapic_addr, vspace_offset);

        Self::enable_io_apic(io_apic_addr);
        Self::enable_lapic(lapic_addr);

        log::info!(
            "apic, lapic initialized at IOAPIC={:x} LAPIC={:x}",
            io_apic_addr.as_u64(),
            lapic_addr.as_u64(),
        );

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
            let lvtt = ptr.offset(ApicRegister::LapicLvtt as isize / 4);
            lvtt.write_volatile(0x20 | ((periodic as u32) << 17));
            let tdcr = ptr.offset(ApicRegister::LapicTdcr as isize / 4);
            tdcr.write_volatile(ApicValue::TdcrDivideBy1 as u32);
            let ticr = ptr.offset(ApicRegister::LapicTicr as isize / 4);
            ticr.write_volatile(ticks);
        }
        ticks
    }

    pub fn read_counter(&self) -> u32 {
        let ptr = self.lapic_addr.as_mut_ptr::<u32>();
        unsafe {
            ptr.add(ApicRegister::LapicTccr as usize / 4)
                .read_volatile()
        }
    }

    pub fn end_interrupt(&self) {
        let ptr = self.lapic_addr.as_mut_ptr::<u32>();
        unsafe {
            ptr.offset(ApicRegister::LapicEoi as isize / 4)
                .write_volatile(0);
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
