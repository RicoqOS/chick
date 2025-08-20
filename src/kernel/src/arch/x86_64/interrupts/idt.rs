use lazy_static::lazy_static;
use x86_64::structures::idt::{
    InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode,
};

use super::gdt::IstIndex;
use crate::{APIC, TICKS};

lazy_static! {
    pub(super) static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // Reserved vectors.
        idt.breakpoint.set_handler_fn(breakpoint_exception);
        unsafe {
            idt.non_maskable_interrupt
                .set_handler_fn(non_maskable_interrupt)
                .set_stack_index(IstIndex::NonMaskableInterrupt as u16);
        }
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault)
                .set_stack_index(IstIndex::DoubleFault as u16);
        }
        idt.page_fault.set_handler_fn(page_fault);
        unsafe {
            idt.machine_check
                .set_handler_fn(machine_check)
                .set_stack_index(IstIndex::MachineCheck as u16);
        }

        // Custom vectors.
        idt[0x20].set_handler_fn(timer_handler);

        idt
    };
}

extern "x86-interrupt" fn breakpoint_exception(
    stack_frame: InterruptStackFrame,
) {
    log::error!("Breakpoint (#BP) Exception: {stack_frame:?}");
}

extern "x86-interrupt" fn non_maskable_interrupt(
    stack_frame: InterruptStackFrame,
) {
    panic!("NMI: {:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("Double fault: {:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    let pfla = match Cr2::read() {
        Ok(addr) => addr.as_u64(),
        Err(_) => 0,
    };

    log::error!(
        "Page fault ({:?}) at {:?}: {:?}",
        error_code,
        pfla,
        stack_frame
    );
}

extern "x86-interrupt" fn machine_check(
    stack_frame: InterruptStackFrame,
) -> ! {
    panic!("Machine check: {:#?}", stack_frame);
}

extern "x86-interrupt" fn timer_handler(_stack_frame: InterruptStackFrame) {
    TICKS.lock().tick_handler();
    APIC.lock().end_interrupt();
}
