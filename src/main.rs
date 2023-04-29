#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(unwrap_infallible)]

use panic_halt as _;

mod display;
mod tcounter;

use arduino_hal::i2c::Direction;
use fixed::{types::extra::U8, FixedU64};
use ufmt_float::uFmt_f32;

use display::I2cDisplay;
use tcounter::TCounter;

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

    let counter = TCounter::new(dp.TC1, true);
    let mut display = I2cDisplay::new(&mut i2c, 0x27u8, &mut serial);
    display.init().unwrap();

    //From this point on an interrupt can happen
    unsafe { avr_device::interrupt::enable() };

    let delay_in_ms: u16 = 200;
    let micros_elapsed: FixedU64<U8> = FixedU64::<U8>::from(1000 * delay_in_ms as u32);

    let mut last_clock_cycles_meas: u32 = 0;

    loop {
        display.clear_display().unwrap();

        let (busy, ac) = display.read_busy_and_AC().unwrap();
        ufmt::uwriteln!(&mut serial, "{} {}", busy, ac).unwrap();

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
