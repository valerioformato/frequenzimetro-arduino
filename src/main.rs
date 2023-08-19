#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(unwrap_infallible)]

use core::str::FromStr;

use panic_halt as _;

mod display;
mod tcounter;

use fixed::{types::extra::U8, FixedU64};
use ufmt_float::uFmt_f32;

use display::I2cDisplay;
use heapless::String;
use tcounter::TCounter;

fn correct_frequency_counts(counts: u32) -> u32 {
    counts - counts * 4 / 100
}

fn get_frequency(
    mut counts: FixedU64<U8>,
    interval_micros: FixedU64<U8>,
) -> (FixedU64<U8>, &'static str) {
    const UNITS: [&str; 3] = ["MHz", "kHz", "Hz"];

    let mut idx = 0;

    while counts < interval_micros {
        counts *= 1000;
        idx += 1;
    }

    (counts / interval_micros, UNITS[idx])
}

fn format_freq(freq: FixedU64<U8>) -> String<7> {
    const DIGITS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
    let powers_of_ten: [FixedU64<U8>; 3] = [
        FixedU64::<U8>::from(1_u32),
        FixedU64::<U8>::from(10_u32),
        FixedU64::<U8>::from(100_u32),
    ];

    let mut result = String::<7>::new();

    for idx in 2..=0 {
        let digit_idx = (freq / powers_of_ten[idx]).floor().to_num();
        match digit_idx {
            0..=9 => Some(DIGITS[digit_idx]),
            _ => None,
        }
        .and_then(|c| Some(result.push(c).unwrap()));
    }

    result
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

    let counter = TCounter::new(dp.TC1, true);
    let mut display = I2cDisplay::new(&mut i2c, 0x27u8);
    ufmt::uwriteln!(&mut serial, "Display created").unwrap();
    display.init().expect("Err initializing display");
    ufmt::uwriteln!(&mut serial, "Display initialized").unwrap();

    display
        .write_string(String::<32>::from_str("Initialized").unwrap())
        .unwrap();

    arduino_hal::delay_ms(500);

    display.clear();
    display
        .write_string(String::<32>::from_str("Frequency:").unwrap())
        .unwrap();

    //From this point on an interrupt can happen
    unsafe { avr_device::interrupt::enable() };

    let delay_in_ms: u16 = 200;
    let micros_elapsed: FixedU64<U8> = FixedU64::<U8>::from(1000 * delay_in_ms as u32);

    let mut last_clock_cycles_meas: u32 = 0;

    loop {
        let clock_cycles_meas = correct_frequency_counts(counter.clock_cycles());
        let delta_clock_cycles: FixedU64<U8> =
            FixedU64::<U8>::from(clock_cycles_meas - last_clock_cycles_meas);

        let (freq, f_unit) = get_frequency(delta_clock_cycles, micros_elapsed);

        let d_disp = uFmt_f32::Three(delta_clock_cycles.to_num::<f32>());
        let f_disp = uFmt_f32::Three(freq.to_num::<f32>());

        ufmt::uwriteln!(
            &mut serial,
            "measured {} clock cycles, freq = {} {}",
            d_disp,
            f_disp,
            f_unit,
        )
        .unwrap();

        // move cursor to second line
        display.move_cursor(16).unwrap();

        last_clock_cycles_meas = clock_cycles_meas;

        arduino_hal::delay_ms(delay_in_ms);
    }
}
