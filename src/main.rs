#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]
#![feature(const_option)]
#![feature(ptr_const_cast)]

mod timerclock;

use core::{
    ptr::null,
    sync::atomic::{AtomicBool, Ordering},
};
use fixed::{types::extra::U3, FixedU32};
use panic_halt as _;
use portable_atomic::{AtomicU32, AtomicUsize, AtomicPtr};
use timerclock::{Resolution, TClock};
use ufmt_float::uFmt_f32;

static PIN_CHANGED: AtomicBool = AtomicBool::new(false);
static INDEX: AtomicUsize = AtomicUsize::new(0);
const MAX_TIME_MEASUREMENTS: usize = 100;
const ZERO_U32: AtomicU32 = AtomicU32::new(0);
static TIME_MEASUREMENTS: [AtomicU32; MAX_TIME_MEASUREMENTS] = [ZERO_U32; MAX_TIME_MEASUREMENTS];
static CLOCK_PTR: AtomicPtr<TClock> = AtomicPtr::new(null::<TClock>().as_mut());

fn average(numbers: &[u32]) -> FixedU32<U3> {
    let sum_it = numbers.iter().filter(|f| **f > 0);
    let count_it = numbers.iter().filter(|f| **f > 0);
    FixedU32::<U3>::from_num(sum_it.sum::<u32>()) / count_it.count() as u32
}

//This function is called on change of pin 2
#[avr_device::interrupt(atmega328p)]
#[allow(non_snake_case)]
fn PCINT2() {
    static mut last_timer_value: u32 = 0;

    // get the new timer tick count
    let mut new_timer_value: u32 = 0;
    unsafe {
        let clock = CLOCK_PTR.load(Ordering::SeqCst);
        new_timer_value = (*clock).micros();
    }

    // count how many ticks have passed
    let delta_t = new_timer_value - *last_timer_value;

    // update the measurements array
    let index = INDEX
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
            Some((x + 1) % MAX_TIME_MEASUREMENTS)
        })
        .unwrap();
    TIME_MEASUREMENTS[index].store(delta_t as u32, Ordering::SeqCst);

    // store the last timer tick count
    *last_timer_value = new_timer_value;
}

fn read_times(values: &[AtomicU32; MAX_TIME_MEASUREMENTS]) -> [u32; MAX_TIME_MEASUREMENTS] {
    let mut result: [u32; MAX_TIME_MEASUREMENTS] = [0; MAX_TIME_MEASUREMENTS];
    avr_device::interrupt::free(|_cs| {
        for i in 0..MAX_TIME_MEASUREMENTS {
            result[i] = values[i].load(Ordering::SeqCst);
        }
        result
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

    // Initialize global clock timer
    // IMPORTANT!!!
    // We set the global pointer to the clock **before** enabling interrupts because we wanna make sure
    // that when PCINT2 fires it can "safely" dereference CLOCK_PTR
    // FIXME: Check if it's ok to create a clock before enabling interrupts
    let mut clock = TClock::new(dp.TC0, Resolution::_1_MS).unwrap();
    CLOCK_PTR.store(&mut clock, Ordering::SeqCst);
        //From this point on an interrupt can happen
    unsafe { avr_device::interrupt::enable() };

    let micros_in_sec: FixedU32<U3> = FixedU32::<U3>::from_num(1_000_000);

    loop {
        let mut time: u32 = clock.millis();
    
        if time % 100 == 0 && INDEX.load(Ordering::SeqCst) > 0 {
            let time_measurements = read_times(&TIME_MEASUREMENTS);
            let mean_interval: FixedU32<U3> = average(&time_measurements);
            let freq = micros_in_sec / mean_interval / 2;
            let v = uFmt_f32::Three(freq.to_num::<f32>());
            let t = uFmt_f32::Three(mean_interval.to_num::<f32>());

            ufmt::uwriteln!(&mut serial, "freq: {} Hz, interval: {} us", v, t).unwrap();
        }
    }
}
