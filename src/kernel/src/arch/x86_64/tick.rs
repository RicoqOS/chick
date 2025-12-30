use core::time::Duration;

use crate::arch::apic::Apic;
use crate::arch::pit::{Mode, Pit};

const DEFAULT_TICKS_HZ: f32 = 100.0; // Default to 10ms.
const CALIBRATION_SAMPLES: usize = 10;

fn set_ioapic_pit_interrupt(apic: Apic) {
    let gsi = 2;
    let vector = 0x20; // IDT handler index for timer.
    let dest_apic_id = 0; // CPU0.

    let low_index = 0x10 + gsi * 2;
    let high_index = low_index + 1;

    // Configure targeted CPU.
    apic.ioapic_write(high_index, (dest_apic_id as u32) << 24);

    let low_value = vector as u32;
    apic.ioapic_write(low_index, low_value);
}

/// Tick handler.
#[derive(Debug, Clone)]
pub struct Tick {
    apic: Apic,
    is_calibration: bool,
    ticks: u64,
    duration: Duration,
    lapic_counter: u32,
    calibration: [u32; CALIBRATION_SAMPLES],
    calibration_idx: usize,
}

impl Tick {
    /// Create a new [`Tick`] manager.
    pub fn new() -> Self {
        Self {
            apic: Apic::new(),
            is_calibration: true,
            ticks: 0,
            duration: Duration::from_millis(50),
            lapic_counter: 0,
            calibration: [0; CALIBRATION_SAMPLES],
            calibration_idx: 0,
        }
    }

    /// Handle each ticks from interrupts.
    pub fn tick_handler(&mut self) {
        if self.is_calibration {
            self.end_calibration();
        } else {
            self.ticks += 1;
            unsafe {
                crate::scheduler::SCHEDULER
                    .get()
                    .expect("scheduler not initialized")
                    .get_mut()
                    .preempt()
            };
        }
    }

    /// Start calibration to get CPU cycles per millisecond.
    pub fn calibrate(mut self, apic: Apic) -> Self {
        log::debug!("initializing calibration...");
        self.is_calibration = true;
        self.apic = apic;
        self.calibration_idx = 0;

        set_ioapic_pit_interrupt(self.apic);

        self.init_counters();

        self
    }

    #[inline(always)]
    fn init_counters(&mut self) {
        // Create a PIT one-shot 10ms counter.
        let mut pit = Pit::new(self.duration);
        pit.set_mode(Mode::InterruptOnTerminalCount);

        // Create an APIC counter.
        // Must not be 0 when PIT finish.
        self.apic.init_counter(false, u32::MAX);
        self.lapic_counter = self.apic.read_counter();
    }

    fn end_calibration(&mut self) {
        let end = self.apic.read_counter();
        let interval = self.lapic_counter - end;

        let hz_to_millis = 1.0 / DEFAULT_TICKS_HZ * 1000.0;
        let cycles_per_ms = interval / self.duration.as_millis() as u32;

        let cycles = cycles_per_ms as f32 * hz_to_millis;

        if self.calibration_idx < CALIBRATION_SAMPLES {
            self.calibration[self.calibration_idx] = cycles as u32;
            self.calibration_idx += 1;
        }

        if self.calibration_idx >= CALIBRATION_SAMPLES {
            let sum: u32 = self.calibration.iter().sum();
            let cycles_mean = sum / CALIBRATION_SAMPLES as u32;

            log::info!(
                "lapic timer is set to {cycles_mean} cycles for {DEFAULT_TICKS_HZ}Hz"
            );

            self.apic.init_counter(true, cycles_mean);
            self.is_calibration = false;
        } else {
            self.init_counters();
        }
    }
}

impl Default for Tick {
    fn default() -> Self {
        Self::new()
    }
}
