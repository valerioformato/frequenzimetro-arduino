// Works for ATMega328p

use core::cell::Cell;
use arduino_hal::pac::TC1;
use avr_device::interrupt::Mutex;

static OVERFLOW_COUNTER: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

pub struct TCounter {
    /// The timer register, gives this instance unique control over it.
    tc1: TC1,
}

impl TCounter {
    pub fn new(tc1: TC1, use_pin_d5: bool) -> TCounter {
        avr_device::interrupt::free(|cs| {
            OVERFLOW_COUNTER.borrow(cs).set(0);
        });

        // set the timer/counter in normal mode
        tc1.tccr1a.write(|w| w.wgm1().bits(0));

        if use_pin_d5 {
            // set clock source to external clock on rising edge
            tc1.tccr1b.write(|w| w.cs1().ext_rising())
        } else {
            // set clock source to CPU clock
            tc1.tccr1b.write(|w| w.cs1().direct())
        }

        // enable counter overflow interrupt
        tc1.timsk1.write(|w| w.toie1().set_bit());

        Self { tc1 }
    }

    pub fn clock_cycles(&self) -> u32 {
        let (mut m, t, ov1) = avr_device::interrupt::free(|cs| {
            let m: u32 = OVERFLOW_COUNTER.borrow(cs).get().into();

            let (t, ov1) = {
                let t: u16 = self.tc1.tcnt1.read().bits();
                let ov1: bool = self.tc1.tifr1.read().tov1().bit();

                (t, ov1)
            };

            (m, t, ov1)
        });

        // Check whether a interrupt was pending when we read the counter value,
        // which typically means it wrapped around, without the millis getting
        // incremented, so we do it here manually:
        if ov1 {
            m += 1;
        }

        m * core::u16::MAX as u32 + t as u32
    }
}

#[avr_device::interrupt(atmega328p)]
fn TIMER1_OVF() {
    // We just increment the overflow interrupt counter, because an interrupt
    // happened.
    avr_device::interrupt::free(|cs| {
        let counter_cell = OVERFLOW_COUNTER.borrow(cs);
        let counter = counter_cell.get();
        counter_cell.set(counter + 1);
    });
}
