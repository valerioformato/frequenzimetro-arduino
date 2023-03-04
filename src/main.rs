#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(const_option)]

mod timerclock;

use core::sync::atomic::{AtomicBool, Ordering};
use panic_halt as _;
use timerclock::{Resolution, TClock};
use fixed::{types::extra::U3, FixedU32};
use ufmt_float::uFmt_f32;

static PIN_CHANGED: AtomicBool = AtomicBool::new(false);

fn average(numbers: &[u32]) -> FixedU32<U3> {
    let sum_it = numbers.iter().filter(|f| **f > 0);
    let count_it = numbers.iter().filter(|f| **f > 0);
    FixedU32::<U3>::from_num( sum_it.sum::<u32>() as u32 ) / count_it.count() as u32
}

//This function is called on change of pin 2
#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
fn PCINT2() {
    PIN_CHANGED.store(true, Ordering::SeqCst);
}

fn rotate(flag: &AtomicBool) -> bool {
    avr_device::interrupt::free(|_cs| {
        if flag.load(Ordering::SeqCst) {
            flag.store(false, Ordering::SeqCst);
            true
        } else {
            false
        }
    })
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut led = pins.d13.into_output();

    let clock_pin = pins.d2;

    // Enable the PCINT2 pin change interrupt
    dp.EXINT.pcicr.write(|w| unsafe { w.bits(0b100) });

    // Enable pin change interrupts on PCINT18 which is pin PD2 (= d2)
    dp.EXINT.pcmsk2.write(|w| w.bits(0b100));

    //From this point on an interrupt can happen
    unsafe { avr_device::interrupt::enable() };

    let clock = TClock::new(dp.TC0, Resolution::_1_MS).unwrap();
    let mut last_timer_value = 0;

    const MAX_TIME_MEASUREMENTS: usize = 100;
    let mut index = 0;
    let mut time_measurements: [u32; MAX_TIME_MEASUREMENTS] = [0; MAX_TIME_MEASUREMENTS];
    let mut array_full = false;

    loop {
        if rotate(&PIN_CHANGED) {
            let new_timer_value = clock.micros();
            match clock_pin.is_high() {
                true => {
                    led.set_high();
                }
                false => {
                    led.set_low();
                }   
            };
            let delta_t = new_timer_value - last_timer_value;
            time_measurements[index] = delta_t as u32;
            index = (index + 1) % MAX_TIME_MEASUREMENTS;

            if index == 0 && !array_full {
                array_full = true;
            }

            last_timer_value = new_timer_value;

            //ufmt::uwriteln!(&mut serial, "Pin status changed after {} us", delta_t).unwrap();
        }

        // every 10 ms
        if clock.millis() % 1000 == 0 && index > 0 {
            let mean_interval: FixedU32::<U3> = average(&time_measurements);
            let v = uFmt_f32::Three(mean_interval.to_num::<f32>());

            ufmt::uwriteln!(&mut serial, "{} us", v).unwrap();
        }
    }
}
