/// IST vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IstIndex {
    DoubleFault = 0,
    NonMaskableInterrupt = 1,
    MachineCheck = 2,
}

/// IDT vectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdtIndex {
    Timer = 0x20,
}