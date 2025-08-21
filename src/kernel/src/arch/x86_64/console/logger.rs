use core::fmt::Write;

use bootloader_api::info::FrameBufferInfo;
use spin::{Mutex, Once};

use crate::arch::console::framebuffer::FrameBufferWriter;

pub static LOGGER: Once<Logger> = Once::new();

/// A logger instance protected by a spinlock.
#[derive(Debug)]
pub struct Logger {
    /// Locked framebuffer writer.
    pub framebuffer: Mutex<FrameBufferWriter>,
}

impl Logger {
    /// Create a new [`Logger`].
    pub fn new(framebuffer: &'static mut [u8], info: FrameBufferInfo) -> Self {
        Self {
            framebuffer: Mutex::new(FrameBufferWriter::new(framebuffer, info)),
        }
    }
}

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut framebuffer = self.framebuffer.lock();
        writeln!(framebuffer, "{:5}: {}", record.level(), record.args())
            .unwrap();
    }

    fn flush(&self) {}
}
