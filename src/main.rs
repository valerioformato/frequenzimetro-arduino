#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::sync::atomic::{AtomicBool, Ordering};
use panic_halt as _;

static PIN_CHANGED: AtomicBool = AtomicBool::new(false);

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

    loop {
        if rotate(&PIN_CHANGED) {
            let new_state = match clock_pin.is_high() {
                true => {
                    led.set_high();
                    'H'
                }
                false => {
                    led.set_low();
                    'L'
                }
            };

            ufmt::uwriteln!(&mut serial, "D2 Status changed to {}", new_state).unwrap();
        }
    }
}
