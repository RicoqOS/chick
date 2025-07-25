//! Based on [bootloader logger](https://github.com/rust-osdev/bootloader/blob/main/common/src/logger.rs).

use bootloader_api::info::FrameBuffer;
use bootloader_boot_config::LevelFilter;
use bootloader_x86_64_common::init_logger;

/// Initializes logger using [`FrameBuffer`].
pub fn init(framebuffer: FrameBuffer) {
    let info = framebuffer.info();
    let buffer = framebuffer.into_buffer();

    init_logger(buffer, info, LevelFilter::Info, true, true);

    log::info!("logger initialized");
}
