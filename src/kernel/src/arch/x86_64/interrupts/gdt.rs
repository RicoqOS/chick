use core::ptr::addr_of;

use spin::Lazy;
use x86_64::VirtAddr;
use x86_64::registers::segmentation::{DS, ES, FS, GS, SS, SegmentSelector};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;

use crate::arch::constants::interrupts::IstIndex;

const STACK_SIZE: usize = 4096 * 5;

/// Task state segment.
pub static TSS: Lazy<TaskStateSegment> = Lazy::new(|| {
    let mut tss = TaskStateSegment::new();

    // We MUST avoid using same stack.
    tss.interrupt_stack_table[IstIndex::DoubleFault as usize] = {
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let stack_start = VirtAddr::from_ptr(addr_of!(STACK));
        stack_start + STACK_SIZE as u64
    };

    tss.interrupt_stack_table[IstIndex::NonMaskableInterrupt as usize] = {
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let stack_start = VirtAddr::from_ptr(addr_of!(STACK));
        stack_start + STACK_SIZE as u64
    };

    tss.interrupt_stack_table[IstIndex::MachineCheck as usize] = {
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let stack_start = VirtAddr::from_ptr(addr_of!(STACK));
        stack_start + STACK_SIZE as u64
    };

    // Privilege stack table for userland calls.
    tss.privilege_stack_table[0] = {
        static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
        let stack_start = VirtAddr::from_ptr(addr_of!(STACK));
        stack_start + STACK_SIZE as u64
    };

    tss
});

pub static GDT: Lazy<(GlobalDescriptorTable, Selectors)> = Lazy::new(|| {
    let mut gdt = GlobalDescriptorTable::new();
    let code_selector = gdt.append(Descriptor::kernel_code_segment());
    let data_selector = gdt.append(Descriptor::kernel_data_segment());
    let user_code = gdt.append(Descriptor::user_code_segment());
    let user_data = gdt.append(Descriptor::user_data_segment());
    let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
    (gdt, Selectors {
        code_selector,
        data_selector,
        user_code_selector: user_code,
        user_data_selector: user_data,
        tss_selector,
    })
});

/// Kernel segment selectors.
#[derive(Debug)]
pub struct Selectors {
    pub code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

/// Loads the GDT into the CPU.
pub fn load() {
    use x86_64::instructions::segmentation::{CS, Segment};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();

    // Flat model.
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        SS::set_reg(GDT.1.data_selector);
        DS::set_reg(GDT.1.data_selector);
        ES::set_reg(GDT.1.data_selector);
        FS::set_reg(GDT.1.data_selector);
        GS::set_reg(GDT.1.data_selector);

        load_tss(GDT.1.tss_selector);
    }
}
