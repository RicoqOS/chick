use alloc::vec::Vec;
use core::time::Duration;

use crate::arch::apic::Apic;
use crate::arch::pit::{Mode, Pit};

const DEFAULT_TICKS_HZ: f32 = 100.0; // Default to 10ms.

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
    calibration: Vec<u32>,
}

impl Tick {
    /// Create a new [`Tick`] manager.
    pub fn new() -> Self {
        // It is safe to create APIC because it points to invalid virtual
        // address. Safe only if APIC is set somewhere after.
        Self {
            apic: Apic::new(),
            is_calibration: true,
            ticks: 0,
            duration: Duration::from_millis(50),
            lapic_counter: 0,
            calibration: Vec::with_capacity(20),
        }
    }

    /// Handle each ticks from interrupts.
    pub fn tick_handler(&mut self) {
        if self.is_calibration {
            self.end_calibration();
        } else {
            self.ticks += 1;
            crate::scheduler::SCHEDULER
                .get()
                .expect("scheduler not initialized")
                .get_mut()
                .preempt();
        }
    }

    /// Start calibration to get CPU cycles per millisecond.
    pub fn calibrate(mut self, apic: Apic) -> Self {
        log::debug!("initializing calibration...");
        self.is_calibration = true;
        self.apic = apic;

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

        log::debug!(
            "lapic timer elapsed {interval} CPU cycles during {}ms",
            self.duration.as_millis()
        );

        // Init periodic LAPIC timer for kernel ticks.
        let hz_to_millis = 1.0 / DEFAULT_TICKS_HZ * 1000.0;
        let cycles_per_ms = interval / self.duration.as_millis() as u32;
        log::debug!("cpu is {cycles_per_ms} cycles per ms");

        // Compensate for certain software slowdowns.
        let cycles = cycles_per_ms as f32 * hz_to_millis;
        self.calibration.push(cycles as u32);

        if self.calibration.len() >= 20 {
            let cycles_mean = self.calibration.iter().sum::<u32>() /
                self.calibration.len() as u32;
            log::info!(
                "lapic timer is set to {cycles_mean} cycles for {DEFAULT_TICKS_HZ}Hz"
            );
            self.apic.init_counter(true, cycles_mean);
            self.is_calibration = false;
            self.calibration.clear();
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
