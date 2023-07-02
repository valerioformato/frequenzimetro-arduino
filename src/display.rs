use core::any::TypeId;
use core::iter::Repeat;

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
    const ENABLE_BIT: u8 = 1 << 2;

    pub fn new(i2c: &'a mut I2c, address: u8) -> Self {
        return Self { i2c, address };
    }

    pub fn init(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        self.write_cmd_imp(FunctionSet::default())
            .and_then(|_a| self.write_cmd_imp(ClearDisplay {}))
            .and_then(|_a| self.write_cmd_imp(ReturnHome {}))
    }

    pub fn read_busy_and_AC(
        i2c: &'a mut arduino_hal::I2c,
    ) -> Result<(bool, u8), arduino_hal::i2c::Error> {
        let mut read_buffer: [u8; 2] = [0; 2];
        let (one, two) = read_buffer.split_at_mut(1);

        // FIXME: check if the command needs two 4-bit transfers or can read with just one

        // read BF and AC command
        let write_buffer: [u8; 3] = [0xA, 0xE, 0xA];
        for value in write_buffer {
            i2c.write(0x27, &[value])?;
            arduino_hal::delay_us(200);

            if (value & Self::ENABLE_BIT) > 0 {
                i2c.read(0x27, one)?;
            }
        }

        for value in write_buffer {
            i2c.write(0x27, &[value])?;
            arduino_hal::delay_us(200);

            if (value & Self::ENABLE_BIT) > 0 {
                i2c.read(0x27, two)?;
            }
        }

        let ac = (read_buffer[0] & 0x70) | (read_buffer[1] & 0xF0) >> 4;
        return Ok(((read_buffer[0] & 0b10000000) != 0, ac));
    }

    fn write_cmd_imp<C: Command + 'static>(
        &mut self,
        cmd: C,
    ) -> Result<(), arduino_hal::i2c::Error> {
        let rs_bit: u8 = match <C>::rs() {
            RS::Enabled => 1,
            RS::Disabled => 0,
        };

        let rw_bit: u8 = match <C>::rw() {
            RW::Read => 1,
            RW::Write => 0,
        };

        let repeat_upper = TypeId::of::<C>() == TypeId::of::<FunctionSet>();

        let payload = cmd.payload();

        let d1 = payload & 0xF0;
        let d2 = payload & 0x0F << 4;

        let s1: u8 = rs_bit | rw_bit << 1;
        let s2: u8 = rs_bit | rw_bit << 1 | Self::ENABLE_BIT;

        // This is the tricky part:
        // - The first write sets up the 4 data pins to their desired state.
        // - The second part switches on the `enable` bit on the display so it will read the data pins.
        // - The third write just resets the `enable` pin on the display to leave it in a known stable state.
        let mut buffer: [u8; 3] = [d1 | s1, d1 | s2, d1 | s1];
        for byte in buffer {
            self.i2c.write(self.address, &[byte])?;
            arduino_hal::delay_us(200);
        }

        // NOTE: FunctionSet requires the upper 4 bytes to be sent twice, since the device starts in 8bit mode
        // This is what the manual says, and even if I'm not quite sure why, this is the way to make it work.
        if repeat_upper {
            for byte in buffer {
                self.i2c.write(self.address, &[byte])?;
                arduino_hal::delay_us(200);
            }
        }
        // TODO: does this also work?
        // self.i2c.write(self.address, &byte)?;

        buffer = [d2 | s1, d2 | s2, d2 | s1];
        for byte in buffer {
            self.i2c.write(self.address, &[byte])?;
            arduino_hal::delay_us(200);
        }

        Ok(())
    }
}
