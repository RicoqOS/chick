/// LAPIC and IOAPIC registers and values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ApicRegister {
    /// Enables the LAPIC and defines the spurious interrupt vector.
    LapicSivr = 0xF0,
    /// End of interrupt register (EOI).
    LapicEoi = 0xB0,

    /// Local vector table timer (LVTT).
    LapicLvtt = 0x320,
    /// Timer divide configuration register (TDCR).
    LapicTdcr = 0x3E0,
    /// Timer initial count register (TICR).
    LapicTicr = 0x380,
    /// Timer current count register (TCCR).
    LapicTccr = 0x390,

    /// IOAPIC identification register.
    IoApicId = 0x0,
    /// IOAPIC version register.
    IoapicVersion = 0x1,
    /// IOAPIC arbitration ID register.
    IoapicArbId = 0x2,
    /// IOAPIC redirection table base.
    IoapicRedirectionTableBase = 0x10,
}

/// Specific APIC and IOAPIC registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ApicValue {
    /// Divide by 1 TDCR value.
    TdcrDivideBy1 = 0x1,
    /// Enable LAPIC.
    SvrEnable = 0x100,
    /// Base LVTT value; bit 5 (periodic) may be set optionally.
    LvttBase = 0x20,
}
