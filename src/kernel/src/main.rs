//! kernel acts as the primary interface between hardware and software.
//! kernel manages CPU time, allocates memory, and handles interrupts.
//! it ensures secure multitasking and prevents unauthorized access.
#![no_std]
#![no_main]
#![feature(abi_x86_interrupt, naked_functions)]

extern crate alloc;

/// Architecture-specific abstraction.
mod arch;

/// EDF-like scheduler.
mod scheduler;

use bootloader_api::config::Mapping;
use bootloader_api::{BootInfo, BootloaderConfig, entry_point};
use spin::{Lazy, Mutex};
use x86_64::VirtAddr;

pub static BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Mapping::Dynamic);
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
    let mut mm = arch::mm::MemoryManagement::new(physical_memory_offset);
    mm.allocate(&boot_info.memory_regions)
        .expect("failed page allocation");
    let (mut mapper, mut allocator) = mm.get_mapper_and_allocator();

    let rsdp_addr = boot_info
        .rsdp_addr
        .take()
        .expect("Failed to find RSDP address");
    let apic = APIC.lock().init(
        rsdp_addr as usize,
        physical_memory_offset,
        &mut mapper,
        &mut allocator,
    );
    *APIC.lock() = apic;

    // Enable interrupts after disabling PIC.
    arch::interrupts::load();

    let ticks = TICKS.lock().clone().calibrate(apic);
    *TICKS.lock() = ticks;

    scheduler::init_scheduler();

    let executor = scheduler::SCHEDULER.get().unwrap().get_mut();

    #[allow(clippy::empty_loop)]
    executor.spawn(scheduler::Task::new(u64::MAX, async move { loop {} }));

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
