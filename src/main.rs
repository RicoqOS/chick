//! kernel acts as the primary interface between hardware and software.
//! kernel manages CPU time, allocates memory, and handles interrupts.
//! it ensures secure multitasking and prevents unauthorized access.
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

/// Priting.
mod console;

/// Macro-commands.
#[macro_use]
mod macros;

/// Panic.
mod panic;

/// Interrupt descriptor table for CPU interrupts.
mod idt;

mod time;

use bootloader_api::{BootInfo, entry_point};
entry_point!(main);

fn main(boot_info: &'static mut BootInfo) -> ! {
    idt::IDT.load();

    print!("{boot_info:?}");

    loop {}
}
