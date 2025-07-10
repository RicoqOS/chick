use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_exception);
        idt.page_fault.set_handler_fn(page_fault);

        idt
    };
}

/// Push IDT to IDTR.
pub fn load() {
    IDT.load();
    log::info!("interrupts initialized");
}

extern "x86-interrupt" fn breakpoint_exception(stack_frame: InterruptStackFrame) {
    log::error!("Breakpoint (#BP) Exception: {stack_frame:?}");
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
