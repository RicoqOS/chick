use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_exception);

        idt
    };
}

/// Push IDT to IDTR.
pub fn load() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_exception(stack_frame: InterruptStackFrame) {
    print!("\n");
    print!("Breakpoint (#BP) Exception: {stack_frame:?}");
}
