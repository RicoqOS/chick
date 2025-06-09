use spin::Mutex;
use volatile::Volatile;

/// Simple Mutex to write onto console.
pub static WRITER: Mutex<Option<Writer>> = Mutex::new(None);

const VGA_MODE: usize = 0xb8000;
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

type Buffer = [[Volatile<[u8; 2]>; BUFFER_WIDTH]; BUFFER_HEIGHT];

/// Color list for VGA mode.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xA,
    LightCyan = 0xB,
    LightRed = 0xC,
    Pink = 0xD,
    Yellow = 0xE,
    White = 0xF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// VGA mode writer.
#[derive(Debug)]
pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Default for Writer {
    fn default() -> Self {
        Self {
            column_position: 0,
            color_code: ColorCode::new(Color::White, Color::Black),
            buffer: unsafe { &mut *(VGA_MODE as *mut Buffer) },
        }
    }
}

impl Writer {
    /// Print one byte into VGA mode.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                self.buffer[row][col].write([byte, self.color_code.0]);
                self.column_position += 1;
            }
        }
    }

    /// Print a string into VGA mode.
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// Break a line.
    pub fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer[row][col].read();
                self.buffer[row - 1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = [b' ', self.color_code.0];
        for col in 0..BUFFER_WIDTH {
            self.buffer[row][col].write(blank);
        }
    }
}

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_string(s);
        Ok(())
    }
}
