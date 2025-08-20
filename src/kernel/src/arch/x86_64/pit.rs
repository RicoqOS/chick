use core::time::Duration;

use x86_64::instructions::port::Port;

const FREQUENCE_HZ: u64 = 1_193_182; // PIT frequence in Hz.
const COMMAND_PORT: u16 = 0x43;
const DATA_PORT: u16 = 0x40; // Channel 0 data port.

/// PIT mode byte.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum Mode {
    /// One-shot.
    InterruptOnTerminalCount = 0x0,
    /// Periodic signal.
    RateGenerator = 0x2,
}

#[derive(Debug, Clone)]
pub struct Pit {
    command_port: Port<u8>,
    data_port: Port<u8>,
}

impl Pit {
    /// Create a new [`Pit`] reader with a counter.
    pub fn new(counter: Duration) -> Self {
        let mut command_port = Port::new(COMMAND_PORT);
        let mut data_port = Port::new(DATA_PORT);

        let millis = counter.as_millis() as f64;
        let frequence = 1000.0 * 1.0 / millis;
        let reload_value = (FREQUENCE_HZ / frequence as u64) as u16;
        let reload_value_low = (reload_value & 0xFF) as u8;
        let reload_value_high = (reload_value >> 8) as u8;
        log::debug!(
            "pit frequency is {frequence:.3} Hz (= {millis}ms) corresponding to {reload_value} ticks"
        );

        unsafe {
            command_port.write(0b00110000);
            data_port.write(reload_value_low);
            data_port.write(reload_value_high);
        }

        Self {
            command_port,
            data_port,
        }
    }

    /// Set PIT mode.
    pub fn set_mode(&mut self, mode_byte: Mode) {
        unsafe {
            self.command_port.write(mode_byte as u8);
        }
    }

    /// Read reamining time on counter.
    pub fn read(&mut self) -> u16 {
        unsafe {
            // Send latch command to lock.
            self.command_port.write(0x00);

            let low_byte = self.data_port.read();
            let high_byte = self.data_port.read();

            ((high_byte as u16) << 8) | (low_byte as u16)
        }
    }
}
