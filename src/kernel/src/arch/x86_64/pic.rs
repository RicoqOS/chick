use x86_64::instructions::port::Port;

const PIC_1_PORT: u16 = 0x21;
const PIC_2_PORT: u16 = 0xA1;

/// PIC manager.
#[derive(Debug, Clone)]
pub struct Pic {
    pic1: Port<u8>,
    pic2: Port<u8>,
}

impl Pic {
    /// Create a new [`Pic`].
    pub fn new() -> Self {
        let pic1 = Port::new(PIC_1_PORT);
        let pic2 = Port::new(PIC_2_PORT);
        Self { pic1, pic2 }
    }

    /// Hide all interupts from IRQ0 to IRQ15.
    pub fn disable(mut self) {
        unsafe {
            self.pic1.write(0xff);
            self.pic2.write(0xff);
        }
    }
}
