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

/// Architecture-specific abstraction.
mod arch;

use bootloader_api::{BootInfo, entry_point};

entry_point!(main);

fn main(boot_info: &'static mut BootInfo) -> ! {
    print!("{boot_info:?}");
    print!("\n");

    arch::interrupts::load();

    loop {}
}

/// Handle panics.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    print!("\n");
    print!("KERNEL PANIC: {info:?}");
    loop {}
}
