//! kernel acts as the primary interface between hardware and software.
//! kernel manages CPU time, allocates memory, and handles interrupts.
//! it ensures secure multitasking and prevents unauthorized access.
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

/// Architecture-specific abstraction.
mod arch;

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

    /*
    let physical_memory_offset = VirtAddr::new(boot_info.physical_memory_offset.into_option().unwrap());
    let mm = arch::mm::MemoryManagement::new(physical_memory_offset);
    print!("\n{:?}", mm);*/

    #[allow(clippy::empty_loop)]
    loop {}
}

/// Handle panics.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("KERNEL PANIC: {info:?}");
    loop {}
}
