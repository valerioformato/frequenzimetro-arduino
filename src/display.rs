use core::any::TypeId;

use arduino_hal::i2c::Error;
use arduino_hal::prelude::{_embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write};
use arduino_hal::I2c;
use heapless::String;

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
            two_line_mode: true,
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

struct EntryModeSet {
    increment_cursor_position: bool,
    shift_display: bool,
}
impl Default for EntryModeSet {
    fn default() -> Self {
        Self {
            increment_cursor_position: true,
            shift_display: false,
        }
    }
}
impl Command for EntryModeSet {
    fn rw() -> RW {
        RW::Write
    }

    fn rs() -> RS {
        RS::Disabled
    }

    fn payload(self) -> u8 {
        0b00000100u8 | (self.increment_cursor_position as u8) << 1 | self.shift_display as u8
    }
}

enum ShiftDirection {
    Right,
    Left,
}
struct CursorDisplayShift {
    display_shift: bool,
    direction: ShiftDirection,
}
impl Default for CursorDisplayShift {
    fn default() -> Self {
        Self {
            display_shift: false,
            direction: ShiftDirection::Right,
        }
    }
}
impl Command for CursorDisplayShift {
    fn rw() -> RW {
        RW::Write
    }

    fn rs() -> RS {
        RS::Disabled
    }

    fn payload(self) -> u8 {
        let direction_bit = match self.direction {
            ShiftDirection::Right => 0x4 as u8,
            ShiftDirection::Left => 0 as u8,
        };

        0b00010000u8 | (self.display_shift as u8) << 3 | direction_bit
    }
}

struct SetDDRAMAddress {
    address: u8,
}
impl Default for SetDDRAMAddress {
    fn default() -> Self {
        Self { address: 0 as u8 }
    }
}
impl Command for SetDDRAMAddress {
    fn rw() -> RW {
        RW::Write
    }

    fn rs() -> RS {
        RS::Disabled
    }

    fn payload(self) -> u8 {
        // NOTE: if address is larger than 7bit-max we truncate it.
        0b10000000u8 | (self.address & 0x7F)
    }
}

struct WriteToDDRAM {
    data: u8,
}
impl Default for WriteToDDRAM {
    fn default() -> Self {
        Self { data: 0 as u8 }
    }
}
impl Command for WriteToDDRAM {
    fn rw() -> RW {
        RW::Write
    }

    fn rs() -> RS {
        RS::Enabled
    }

    fn payload(self) -> u8 {
        self.data
    }
}

pub struct I2cDisplay<'a> {
    i2c: &'a mut I2c,
    address: u8,
}

impl<'a> I2cDisplay<'a> {
    const ENABLE_BIT: u8 = 1 << 2;
    const ON_BIT: u8 = 1 << 3;

    pub fn new(i2c: &'a mut I2c, address: u8) -> Self {
        return Self { i2c, address };
    }

    pub fn init(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        self.write_cmd_imp(FunctionSet::default())
            .and_then(|_a| self.write_cmd_imp(ClearDisplay {}))
            .and_then(|_a| self.write_cmd_imp(DisplayControls::default()))
            .and_then(|_a| self.write_cmd_imp(EntryModeSet::default()))
            .and_then(|_a| self.write_cmd_imp(ReturnHome {}))
            .and_then(|_a| self.write_cmd_imp(SetDDRAMAddress::default()))
    }

    pub fn clear(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        self.write_cmd_imp(ClearDisplay {})
            .and_then(|_a| self.write_cmd_imp(ReturnHome {}))
    }

    pub fn move_cursor(&mut self, position: u8) -> Result<(), arduino_hal::i2c::Error> {
        let address = match position {
            0..=15 => position,
            16..=32 => (position - 16) + 0x40,
            _ => return Ok(()),
        };

        self.write_cmd_imp(SetDDRAMAddress { address: address })
    }

    pub fn write_string(&mut self, msg: String<32>) -> Result<(), arduino_hal::i2c::Error> {
        for char in msg.as_bytes() {
            self.write_cmd_imp(WriteToDDRAM { data: char.clone() })?;
        }

        Ok(())
    }

    pub fn read_busy_and_AC(&mut self) -> Result<(bool, u8), arduino_hal::i2c::Error> {
        let mut read_buffer: [u8; 2] = [0; 2];
        let (one, two) = read_buffer.split_at_mut(1);

        // FIXME: check if the command needs two 4-bit transfers or can read with just one

        // read BF and AC command
        let write_buffer: [u8; 3] = [0xA, 0xE, 0xA];
        for value in write_buffer {
            self.i2c.write(self.address, &[value])?;
            arduino_hal::delay_us(200);
        }
        self.i2c.read(self.address, one)?;

        for value in write_buffer {
            self.i2c.write(self.address, &[value])?;
            arduino_hal::delay_us(200);
        }
        self.i2c.read(self.address, two)?;

        let ac = (read_buffer[0] & 0x70) | (read_buffer[1] & 0xF0) >> 4;
        return Ok(((read_buffer[0] & 0b10000000) != 0, ac));
    }

    fn expand_cmd_sequence(data: u8) -> [u8; 3] {
        [data, data | Self::ENABLE_BIT, data]
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

        let upper_half_cmd = Self::expand_cmd_sequence(
            (payload & 0xF0) | ((rs_bit as u8) | ((rw_bit as u8) << 1) | Self::ON_BIT),
        );
        let lower_half_cmd = Self::expand_cmd_sequence(
            ((payload & 0xF) << 4) | ((rs_bit as u8) | ((rw_bit as u8) << 1) | Self::ON_BIT),
        );

        let buffer: [u8; 6] = {
            let mut whole: [u8; 6] = [0; 6];
            let (one, two) = whole.split_at_mut(upper_half_cmd.len());
            one.copy_from_slice(&upper_half_cmd);
            two.copy_from_slice(&lower_half_cmd);
            whole
        };

        if repeat_upper {
            for value in upper_half_cmd {
                // We disable the ON bit on the first round, so we effectively power cycle the display if it's already on
                self.i2c.write(self.address, &[value & !Self::ON_BIT])?;
                arduino_hal::delay_us(200);
            }
        }

        for value in buffer {
            self.i2c.write(self.address, &[value])?;
            arduino_hal::delay_us(200);
        }
        while match self.read_busy_and_AC()? {
            (busy, _) => busy,
        } {}

        Ok(())
    }
}
