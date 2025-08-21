/// Framebuffer.
mod framebuffer;

/// Logger.
pub mod logger;

use bootloader_api::info::FrameBuffer;
use log::LevelFilter;

/// Create a new logger based on [`log`].
pub fn init(framebuffer: FrameBuffer) {
    let info = framebuffer.info();
    let buffer = framebuffer.into_buffer();

    let logger =
        logger::LOGGER.call_once(move || logger::Logger::new(buffer, info));

    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    let _ = log::set_logger(logger);
    log::set_max_level(level);
    log::info!("framebuffer : {info:?}");
}
