use x86_64::instructions::port::Port;

use core::time::Duration;

const FREQUENCE_HZ: f32 = 1193182.0; // PIT frequence in Hz.
const LOCK_PORT: u16 = 0x43;
const READ_PORT: u16 = 0x40;

/// PIT mode byte.
#[repr(u8)]
#[derive(Debug)]
pub enum ModeByte {
    /// One-shot.
    InterruptOnTerminalCount = 0x0,
    /// Periodic signal.
    RateGenerator = 0x2,
}

pub struct Pit {
    lock_port: Port<u8>,
    read_port: Port<u8>,
}

impl Pit {
    /// Create a new [`Pit`] reader with a counter.
    pub fn new(counter: Duration) -> Self {
        let lock_port = Port::new(LOCK_PORT);
        let mut read_port = Port::new(READ_PORT);

        let duration = counter.as_millis() as u64 as f32;
        let duration = duration / 1000.0;
        let reload_value = (FREQUENCE_HZ * duration) as u16;

        unsafe {
            read_port.write((reload_value & 0xFF) as u8);
            read_port.write((reload_value >> 8) as u8);
        }

        Self {
            lock_port,
            read_port,
        }
    }

    /// Set PIT mode.
    pub fn set_mode(&mut self, mode_byte: ModeByte) {
        unsafe {
            self.lock_port.write(mode_byte as u8);
        }
    }

    /// Read reamining time on counter.
    pub fn read(&mut self) -> u16 {
        unsafe {
            // Send latch command to lock.
            self.lock_port.write(0x00);

            let low_byte = self.read_port.read();
            let high_byte = self.read_port.read();

            ((high_byte as u16) << 8) | (low_byte as u16)
        }
    }
}
