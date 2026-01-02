//! kernel acts as the primary interface between hardware and software.
//! kernel manages CPU time, allocates memory, and handles interrupts.
//! it ensures secure multitasking and prevents unauthorized access.
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(unsafe_op_in_unsafe_fn, dead_code)]

mod arch;
mod error;
mod objects;
mod scheduler;
mod syscall;
#[macro_use]
mod macros;
mod cspace;
mod vspace;

use bootloader_api::config::Mapping;
use bootloader_api::{BootInfo, BootloaderConfig, entry_point};
use spin::{Lazy, Mutex};
use x86_64::VirtAddr;

pub const KERNEL_STACK_GUARD: u64 = 0xffff_ffff_7000_0000;
pub const BOOT_INFO_ADDR: u64 = 0xffff_ffff_4000_0000;
pub const PHYS_MEM_OFFSET: u64 = 0xffff_8000_0000_0000;
pub const RECURSIVE_P4_ADDR: u64 = 0xffff_ff00_0000_0000;
pub const KERNEL_STACK_SIZE: u64 = 128 * 1024;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();

    // Avoid address randomization.
    config.mappings.aslr = false;

    config.mappings.kernel_stack = Mapping::FixedAddress(KERNEL_STACK_GUARD);
    config.kernel_stack_size = KERNEL_STACK_SIZE;
    config.mappings.boot_info = Mapping::FixedAddress(BOOT_INFO_ADDR);
    config.mappings.physical_memory =
        Some(Mapping::FixedAddress(PHYS_MEM_OFFSET));
    config.mappings.page_table_recursive =
        Some(Mapping::FixedAddress(RECURSIVE_P4_ADDR));

    config
};
pub static APIC: Lazy<Mutex<arch::apic::Apic>> =
    Lazy::new(|| Mutex::new(arch::apic::Apic::new()));
pub static TICKS: Lazy<Mutex<arch::tick::Tick>> =
    Lazy::new(|| Mutex::new(arch::tick::Tick::new()));

entry_point!(main, config = &BOOTLOADER_CONFIG);

fn main(boot_info: &'static mut BootInfo) -> ! {
    #[cfg(feature = "framebuffer")]
    arch::console::init(
        boot_info
            .framebuffer
            .take()
            .expect("framebuffer not usable"),
    );

    let physical_memory_offset = VirtAddr::new(
        boot_info
            .physical_memory_offset
            .into_option()
            .expect("physical memory offset undefined"),
    );

    let rsdp_addr = boot_info
        .rsdp_addr
        .take()
        .expect("Failed to find RSDP address");
    let apic = APIC
        .lock()
        .init(rsdp_addr as usize, physical_memory_offset.as_u64());
    *APIC.lock() = apic;

    // Enable interrupts after disabling PIC.
    arch::interrupts::load();

    let ticks = TICKS.lock().clone().calibrate(apic);
    *TICKS.lock() = ticks;

    scheduler::init_scheduler();

    // Enable syscalls.
    arch::syscall::init_syscall();

    let executor = scheduler::SCHEDULER.get().unwrap().get_mut();
    executor.run()
}

/// Handle panics.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    #[cfg(feature = "framebuffer")]
    if let Some(logger) = arch::console::logger::LOGGER.get() {
        logger.framebuffer.try_lock().unwrap().panic_screen();
    }
    log::error!("KERNEL PANIC: {info:?}");
    loop {}
}
