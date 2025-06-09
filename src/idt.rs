use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

#[derive(Debug, Clone, Copy)]
enum IRQ {
    _Timer = 0,
    PS2 = 1,
}

impl IRQ {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

lazy_static! {
    pub(super) static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(crate::panic::breakpoint_handler);

        idt
    };
}
