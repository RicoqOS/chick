//! kernel acts as the primary interface between hardware and software.
//! kernel manages CPU time, allocates memory, and handles interrupts.
//! it ensures secure multitasking and prevents unauthorized access.
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt, naked_functions)]

/// Architecture-specific abstraction.
mod arch;

/// EDF scheduler.
mod scheduler;

use bootloader_api::config::Mapping;
use bootloader_api::{BootInfo, BootloaderConfig, entry_point};
use x86_64::VirtAddr;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
    config
};

entry_point!(main, config = &BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    arch::console::init(
        boot_info
            .framebuffer
            .take()
            .expect("framebuffer not usable"),
    );
    arch::interrupts::load();

    let physical_memory_offset = VirtAddr::new(
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("physical memory offset undefined"),
    );
    let mm = arch::mm::MemoryManagement::new(physical_memory_offset);
    mm.allocate(&boot_info.memory_regions)
        .expect("failed page allocation");

    let mut executor = scheduler::executor::Executor::new();

    executor.run()
}

/// Handle panics.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("KERNEL PANIC: {info:?}");
    loop {}
}
