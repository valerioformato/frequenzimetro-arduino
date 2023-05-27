use arduino_hal::i2c::Direction;
use arduino_hal::prelude::{_embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write};
use arduino_hal::I2c;

enum RW {
    Read,
    Write,
}

enum RS {
    Disabled,
    Enabled,
}
trait Command {
    fn rw() -> RW;
    fn rs() -> RS;
    fn payload(self) -> u8;
}

struct ClearDisplay {}
impl Command for ClearDisplay {
    fn rw() -> RW {
        RW::Write
    }

    fn rs() -> RS {
        RS::Disabled
    }

    fn payload(self) -> u8 {
        0b00000001u8
    }
}

struct ReturnHome {}
impl Command for ReturnHome {
    fn rw() -> RW {
        RW::Write
    }

    fn rs() -> RS {
        RS::Disabled
    }

    fn payload(self) -> u8 {
        0b00000010u8
    }
}

struct FunctionSet {
    eight_bit_mode: bool,
    two_line_mode: bool,
    big_fonts: bool,
}
impl Default for FunctionSet {
    fn default() -> Self {
        Self {
            eight_bit_mode: false,
            two_line_mode: false,
            big_fonts: false,
        }
    }
}
impl Command for FunctionSet {
    fn rw() -> RW {
        RW::Write
    }

    fn rs() -> RS {
        RS::Disabled
    }

    fn payload(self) -> u8 {
        0b00100000u8
            | (self.eight_bit_mode as u8) << 4
            | (self.two_line_mode as u8) << 3
            | (self.big_fonts as u8) << 2
    }
}

struct DisplayControls {
    display_on: bool,
    cursor_on: bool,
    cursor_blink: bool,
}
impl Default for DisplayControls {
    fn default() -> Self {
        Self {
            display_on: true,
            cursor_on: false,
            cursor_blink: false,
        }
    }
}
impl Command for DisplayControls {
    fn rw() -> RW {
        RW::Write
    }

    fn rs() -> RS {
        RS::Disabled
    }

    fn payload(self) -> u8 {
        0b00001000u8
            | (self.display_on as u8) << 2
            | (self.cursor_on as u8) << 1
            | (self.cursor_blink as u8)
    }
}

pub struct I2cDisplay<'a> {
    i2c: &'a mut I2c,
    address: u8,
}

impl<'a> I2cDisplay<'a> {
    pub fn new(i2c: &'a mut I2c, address: u8) -> Self {
        return Self { i2c, address };
    }

    pub fn init(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        self.write_cmd_imp(ClearDisplay {})
            .and_then(|_a| self.write_cmd_imp(FunctionSet::default()))
            .and_then(|_a| self.write_cmd_imp(ReturnHome {}))
    }

    fn write_cmd_imp<C: Command>(&mut self, cmd: C) -> Result<(), arduino_hal::i2c::Error> {
        const ENABLE_BIT: u8 = 1 << 2;

        let rs_bit: u8 = match <C>::rs() {
            RS::Enabled => 1,
            RS::Disabled => 0,
        };

        let rw_bit: u8 = match <C>::rw() {
            RW::Read => 1,
            RW::Write => 0,
        };

        let payload = cmd.payload();

        let d1 = payload & 0xF0;
        let d2 = payload & 0x0F << 4;

        let s1: u8 = rs_bit | rw_bit << 1;
        let s2: u8 = rs_bit | rw_bit << 1 | ENABLE_BIT;

        // This is the tricky part:
        // - The first write sets up the 4 data pins to their desired state.
        // - The second part switches on the `enable` bit on the display so it will read the data pins.
        // - The third write just resets the `enable` pin on the display to leave it in a known stable state.
        let mut buffer: [u8; 3] = [d1 | s1, d1 | s2, d1 | s1];
        for byte in buffer {
            self.i2c.write(self.address, &[byte])?;
        }
        // TODO: does this also work?
        // self.i2c.write(self.address, &byte)?;

        buffer = [d2 | s1, d2 | s2, d2 | s1];
        for byte in buffer {
            self.i2c.write(self.address, &[byte])?;
        }

        Ok(())
    }
}
