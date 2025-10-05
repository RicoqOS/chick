/// Global descriptor table.
pub mod gdt;

/// Interrupt descriptor table.
mod idt;

/// Push IDT to IDTR.
pub fn load() {
    gdt::load();
    idt::IDT.load();
    x86_64::instructions::interrupts::enable(); // switch cli to sti.
    log::info!("interrupts initialized");
}
