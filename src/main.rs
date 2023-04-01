#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;

mod tcounter;

use fixed::{types::extra::U8, FixedU64};
use ufmt_float::uFmt_f32;

use tcounter::TCounter;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    /*
     * For examples (and inspiration), head to
     *
     *     https://github.com/Rahix/avr-hal/tree/main/examples
     *
     * NOTE: Not all examples were ported to all boards!  There is a good chance though, that code
     * for a different board can be adapted for yours.  The Arduino Uno currently has the most
     * examples available.
     */

    let mut led = pins.d13.into_output();

    let counter = TCounter::new(dp.TC1, false);

    //From this point on an interrupt can happen
    unsafe { avr_device::interrupt::enable() };

    let delay_in_ms: u16 = 5000;
    let micros_elapsed: FixedU64<U8> = FixedU64::<U8>::from(1000 * delay_in_ms as u32);

    let mut last_clock_cycles_meas: u32 = 0;

    loop {
        led.toggle();

        let clock_cycles_meas = counter.clock_cycles();
        let delta_clock_cycles: FixedU64<U8> = FixedU64::<U8>::from(clock_cycles_meas - last_clock_cycles_meas);

        let freq =  delta_clock_cycles / micros_elapsed;

        let d_disp = uFmt_f32::Three(delta_clock_cycles.to_num::<f32>());
        let f_disp = uFmt_f32::Three(freq.to_num::<f32>());

        ufmt::uwriteln!(&mut serial, "measured {} clock cycles, freq = {} MHz", d_disp, f_disp).unwrap();

        last_clock_cycles_meas = clock_cycles_meas;

        arduino_hal::delay_ms(delay_in_ms);
    }
}
