use arduino_hal::i2c::Direction;
use arduino_hal::prelude::{_embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write};
use arduino_hal::I2c;

use ufmt::uWrite;

pub struct I2cDisplay<'a> {
    // The i2c interface for communication
    i2c: &'a mut I2c,
    // The display address
    address: u8,
}

impl<'a> I2cDisplay<'a> {
    pub fn new(i2c: &'a mut I2c, address: u8) -> Self {
        Self { i2c, address }
    }

    pub fn init(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        self.clear_display()
            .and_then(|_a| self.return_home())
            .and_then(|_a| self.function_set())
            .and_then(|_a| self.display_control(false, false))
            .and_then(|_a| self.set_cursor_display_shift(false, true))
    }

    pub fn clear_display(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        self.write_cmd_imp(false, false, 0b00000001)
    }

    fn return_home(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        self.write_cmd_imp(false, false, 0b00000010)
    }

    fn function_set(&mut self) -> Result<(), arduino_hal::i2c::Error> {
        // TODO: allow setting 1/2 line mode, and font size
        self.write_cmd_imp(false, false, 0b00101000)
    }

    fn display_control(
        &mut self,
        display_cursor: bool,
        blink_cursor: bool,
    ) -> Result<(), arduino_hal::i2c::Error> {
        self.write_cmd_imp(
            false,
            false,
            0b00001100 | (display_cursor as u8) << 1 | (blink_cursor as u8),
        )
    }

    fn set_cursor_display_shift(
        &mut self,
        display_shift: bool,
        shift_right: bool,
    ) -> Result<(), arduino_hal::i2c::Error> {
        self.write_cmd_imp(
            false,
            false,
            0b00010000 | (display_shift as u8) << 3 | (shift_right as u8) << 2,
        )
    }

    pub fn read_busy_and_AC(&mut self) -> Result<(bool, u8), arduino_hal::i2c::Error> {
        // read BF and AC command
        let mut buffer: [u8; 2] = [0xA, 0xE];
        self.i2c.write(0x27, &buffer)?;

        self.i2c.read(0x27, &mut buffer)?;

        let ac = buffer[0] & 0x70 | buffer[1] & 0xF0;
        return Ok(((buffer[0] & 0b10000000) != 0, ac));
    }

    fn write_cmd_imp(
        &mut self,
        rs: bool,
        rw: bool,
        data: u8,
    ) -> Result<(), arduino_hal::i2c::Error> {
        let buffer: [u8; 2] = [
            // 4 msb of data, no enable, p3 always set
            (data & 0xF0) | ((rs as u8) | ((rw as u8) << 1) | 0x8),
            // 4 lsb of data, enable on, p3 always set
            ((data & 0xF) << 4) | ((rs as u8) | (rw as u8 >> 1) | 0xC),
        ];

        self.i2c.write(self.address, &buffer)?;

        while match self.read_busy_and_AC()? {
            (busy, ac) => busy,
        } {}

        Ok(())
    }
}
