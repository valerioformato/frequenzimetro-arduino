#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(unwrap_infallible)]

use embedded_hal::blocking::serial::write;
use panic_halt as _;

mod display;
mod tcounter;

use arduino_hal::i2c::Direction;
use fixed::{types::extra::U8, FixedU64};
use ufmt_float::uFmt_f32;

use display::I2cDisplay;
use tcounter::TCounter;

use arduino_hal::prelude::{_embedded_hal_blocking_i2c_Read, _embedded_hal_blocking_i2c_Write};
pub fn read_busy_and_AC(i2c: &mut arduino_hal::I2c) -> Result<(bool, u8), arduino_hal::i2c::Error> {
    const ENABLE_PIN: u8 = 0x4;

    let mut read_buffer: [u8; 2] = [0; 2];
    let (one, two) = read_buffer.split_at_mut(1);

    // read BF and AC command
    let write_buffer: [u8; 3] = [0xA, 0xE, 0xA];
    for value in write_buffer {
        i2c.write(0x27, &[value])?;
        if (value & ENABLE_PIN) > 0 {
            i2c.read(0x27, one)?;
        }
    }

    for value in write_buffer {
        i2c.write(0x27, &[value])?;
        if (value & ENABLE_PIN) > 0 {
            i2c.read(0x27, two)?;
        }
    }

    let ac = (read_buffer[0] & 0x70) | (read_buffer[1] & 0xF0) >> 4;
    return Ok(((read_buffer[0] & 0b10000000) != 0, ac));
}

fn expand_cmd_sequence(data: u8) -> [u8; 3] {
    const ENABLE_PIN: u8 = 0x4;

    [data, data | ENABLE_PIN, data]
}

fn write_cmd_imp(
    i2c: &mut arduino_hal::I2c,
    rs: bool,
    rw: bool,
    data: u8,
    repeat_upper: bool,
) -> Result<(), arduino_hal::i2c::Error> {
    let upper_half_cmd =
        expand_cmd_sequence((data & 0xF0) | ((rs as u8) | ((rw as u8) << 1) | 0x8));
    let lower_half_cmd =
        expand_cmd_sequence(((data & 0xF) << 4) | ((rs as u8) | ((rw as u8) << 1) | 0x8));

    let buffer: [u8; 6] = {
        let mut whole: [u8; 6] = [0; 6];
        let (one, two) = whole.split_at_mut(upper_half_cmd.len());
        one.copy_from_slice(&upper_half_cmd);
        two.copy_from_slice(&lower_half_cmd);
        whole
    };

    if repeat_upper {
        for value in upper_half_cmd {
            i2c.write(0x27, &[value])?;
            arduino_hal::delay_us(200);
        }
    }

    for value in buffer {
        i2c.write(0x27, &[value])?;
        arduino_hal::delay_us(200);
    }
    while match read_busy_and_AC(i2c)? {
        (busy, _) => busy,
    } {}

    Ok(())
}

fn correct_frequency_counts(counts: u32) -> u32 {
    counts - counts * 4 / 100
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut i2c = arduino_hal::I2c::new(
        dp.TWI,
        pins.a4.into_pull_up_input(),
        pins.a5.into_pull_up_input(),
        50000,
    );

    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut _led = pins.d13.into_output();

    // let counter = TCounter::new(dp.TC1, true);
    // let mut display = I2cDisplay::new(&mut i2c, 0x27u8);
    // ufmt::uwriteln!(&mut serial, "Display created").unwrap();
    // display.init();
    // ufmt::uwriteln!(&mut serial, "Display initialized").unwrap();

    // write_cmd_imp(&mut i2c, false, false, 0b00101000).expect("Err Function set");
    write_cmd_imp(&mut i2c, false, false, 0b00101000, true).expect("Err Function set");
    ufmt::uwriteln!(&mut serial, "Function set").unwrap();
    let (bf, ac) = read_busy_and_AC(&mut i2c).expect("Err reading BF and AC");
    ufmt::uwriteln!(&mut serial, "Read BF = {} AC = {}", bf, ac).unwrap();
    arduino_hal::delay_ms(2000);

    write_cmd_imp(&mut i2c, false, false, 0b00000001, false).expect("Err display clear");
    ufmt::uwriteln!(&mut serial, "Display clear").unwrap();
    arduino_hal::delay_ms(2000);

    write_cmd_imp(&mut i2c, false, false, 0b00001111, false).expect("Err display ON");
    ufmt::uwriteln!(&mut serial, "Display ON").unwrap();
    arduino_hal::delay_ms(2000);

    // write_cmd_imp(&mut i2c, false, false, 0b00000010).expect("Err return home");
    // ufmt::uwriteln!(&mut serial, "Return home").unwrap();
    // arduino_hal::delay_ms(2000);

    //From this point on an interrupt can happen
    unsafe { avr_device::interrupt::enable() };

    let delay_in_ms: u16 = 200;
    let micros_elapsed: FixedU64<U8> = FixedU64::<U8>::from(1000 * delay_in_ms as u32);

    let mut last_clock_cycles_meas: u32 = 0;

    let mut buffer: [u8; 256] = [0u8; 256];

    loop {
        write_cmd_imp(&mut i2c, false, false, 0b00000001, false).expect("Err display clear");
        // let (busy, ac) = read_busy_and_AC(&mut i2c).expect("Err read BF and AC");
        // ufmt::uwriteln!(&mut serial, "{} {}", busy, ac).unwrap();
        arduino_hal::delay_ms(100);

        // let (busy, ac) = read_busy_and_AC(&mut i2c).unwrap();
        // ufmt::uwriteln!(&mut serial, "{} {}", busy, ac).unwrap();

        // let clock_cycles_meas = correct_frequency_counts(counter.clock_cycles());
        // let delta_clock_cycles: FixedU64<U8> =
        //     FixedU64::<U8>::from(clock_cycles_meas - last_clock_cycles_meas);

        // let freq = delta_clock_cycles / micros_elapsed;

        // let d_disp = uFmt_f32::Three(delta_clock_cycles.to_num::<f32>());
        // let f_disp = uFmt_f32::Three(freq.to_num::<f32>());

        // ufmt::uwriteln!(
        //     &mut serial,
        //     "measured {} clock cycles, freq = {} MHz",
        //     d_disp,
        //     f_disp
        // )
        // .unwrap();

        // last_clock_cycles_meas = clock_cycles_meas;

        // arduino_hal::delay_ms(delay_in_ms);
    }
}
