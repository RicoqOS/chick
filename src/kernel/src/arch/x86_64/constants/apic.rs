/// Enum des registres et valeurs pour LAPIC et IOAPIC afin d'éviter les magic numbers.
/// Chaque variante est documentée pour expliquer son usage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ApicRegister {
    // --- LAPIC Registers ---

    /// Register du Spurious Interrupt Vector (SVR).
    /// Sert à activer le LAPIC et à définir le vecteur d'interruption pour les interruptions "spurious".
    LapicSivr = 0xF0,

    /// Register du Local Vector Table Timer (LVTT).
    /// Permet de configurer le timer local du LAPIC (mode, vecteur...).
    LapicLvtt = 0x320,

    /// Register du Timer Divide Configuration (TDCR).
    /// Permet de configurer le diviseur de fréquence du timer.
    LapicTdcr = 0x3E0,

    /// Register du Timer Initial Count (TICR).
    /// Permet de charger la valeur initiale du compteur du timer.
    LapicTicr = 0x380,

    /// Register du Timer Current Count (TCCR).
    /// Permet de lire la valeur courante du compteur du timer.
    LapicTccr = 0x390,

    /// Register End Of Interrupt (EOI).
    /// Sert à signaler la fin du traitement d'une interruption au LAPIC.
    LapicEoi = 0xB0,

    // --- IOAPIC Registers ---

    /// Register IOAPIC Identification.
    /// Sert à lire l'identifiant de l'IOAPIC.
    IoapicId = 0x0,

    /// Register IOAPIC Version.
    /// Sert à lire la version de l'IOAPIC.
    IoapicVersion = 0x1,

    /// Register IOAPIC Arbitration ID.
    /// Sert à lire l'identifiant d'arbitrage de l'IOAPIC.
    IoapicArbId = 0x2,

    /// Register IOAPIC Redirection Table.
    /// Base pour configurer le mapping des interruptions externes.
    IoapicRedirectionTableBase = 0x10,
}

/// Enum des valeurs spécifiques pour certains registres APIC/IOAPIC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ApicValue {
    /// Valeur à écrire dans TDCR (Timer Divide Configuration Register) pour diviser par 1.
    TdcrDivideBy1 = 0x1,

    /// Bit à positionner dans SVR pour activer le LAPIC.
    SvrEnable = 0x100,

    /// Valeur d'activation typique pour le LVTT, avec le bit 5 (periodic) optionnel.
    LvttBase = 0x20,
}