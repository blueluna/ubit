#![no_std]
#![no_main]

extern crate panic_halt;
extern crate cortex_m;
extern crate cortex_m_rt;
#[macro_use]
extern crate microbit;
extern crate microbit_radio;

use core::cell::RefCell;
use core::fmt::Write;
use core::ops::DerefMut;

use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;

use microbit::hal::prelude::*;
use microbit::hal::serial;
use microbit::hal::serial::BAUD115200;

// use microbit_radio::Radio;
use microbit_radio::LedDisplay;

const LITTLE_HEART: [[u8; 5]; 5] = [
    [0, 0, 0, 0, 0],
    [0, 1, 0, 1, 0],
    [0, 1, 1, 1, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
];

const HEART: [[u8; 5]; 5] = [
    [0, 1, 0, 1, 0],
    [1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1],
    [0, 1, 1, 1, 0],
    [0, 0, 1, 0, 0],
];

const CLEAR: [[u8; 5]; 5] = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

const MID_DOTT: [[u8; 5]; 5] = [
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 1, 0, 0],
    [0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0],
];

// static RADIO: Mutex<RefCell<Option<Radio>>> = Mutex::new(RefCell::new(None));
static TIMER: Mutex<RefCell<Option<microbit::TIMER0>>> = Mutex::new(RefCell::new(None));
static RTC: Mutex<RefCell<Option<microbit::RTC0>>> = Mutex::new(RefCell::new(None));
static DISPLAY: Mutex<RefCell<Option<LedDisplay>>> = Mutex::new(RefCell::new(None));
static STATE: Mutex<RefCell<Option<u8>>> = Mutex::new(RefCell::new(None));
static TX: Mutex<RefCell<Option<serial::Tx<microbit::UART0>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    if let Some(p) = microbit::Peripherals::take() {
        // Configure high frequency clock to 16MHz
        p.CLOCK.xtalfreq.write(|w| w.xtalfreq()._16mhz());
        p.CLOCK.tasks_hfclkstart.write(|w| unsafe { w.bits(1) });
        while p.CLOCK.events_hfclkstarted.read().bits() == 0 {}
        // Configure low frequency clock to 32.768 kHz
        p.CLOCK.tasks_lfclkstart.write(|w| unsafe { w.bits(1) });
        while p.CLOCK.events_lfclkstarted.read().bits() == 0 {}
        p.CLOCK.events_lfclkstarted.write(|w| unsafe { w.bits(0) });

        cortex_m::interrupt::free(move |cs| {
            let gpio = p.GPIO.split();
            // let mut radio = Radio::new(p.RADIO);

            // Display
            let row1 = gpio.pin13.into_push_pull_output();
            let row2 = gpio.pin14.into_push_pull_output();
            let row3 = gpio.pin15.into_push_pull_output();
            let col1 = gpio.pin4.into_push_pull_output();
            let col2 = gpio.pin5.into_push_pull_output();
            let col3 = gpio.pin6.into_push_pull_output();
            let col4 = gpio.pin7.into_push_pull_output();
            let col5 = gpio.pin8.into_push_pull_output();
            let col6 = gpio.pin9.into_push_pull_output();
            let col7 = gpio.pin10.into_push_pull_output();
            let col8 = gpio.pin11.into_push_pull_output();
            let col9 = gpio.pin12.into_push_pull_output();

            // Configure RX and TX pins accordingly
            let tx = gpio.pin24.into_push_pull_output().downgrade();
            let rx = gpio.pin25.into_floating_input().downgrade();
            let (mut serial_tx, _) = serial::Serial::uart0(p.UART0, tx, rx, BAUD115200).split();

            let _ = write!(serial_tx, "\n\rStarting!\n\r");

            let display = LedDisplay::new(
                col1, col2, col3, col4, col5, col6, col7, col8, col9, row1, row2, row3,
            );
            // Configure RTC with 125 ms resolution 
            p.RTC0.prescaler.write(|w| unsafe { w.bits(4095) });
            // Enable interrupt for tick
            p.RTC0.intenset.write(|w| w.tick().set_bit());
            // Start counter
            p.RTC0.tasks_start.write(|w| unsafe { w.bits(1) });

            // Configure a timer with 1us resolution
            p.TIMER0.bitmode.write(|w| w.bitmode()._32bit());
            p.TIMER0.prescaler.write(|w| unsafe { w.prescaler().bits(4) });
            p.TIMER0.intenset.write(|w| w.compare0().set());
            p.TIMER0.shorts.write(|w| w.compare0_clear().enabled()
                .compare0_stop().enabled());
            p.TIMER0.cc[0].write(|w| unsafe { w.bits(2000) });
            p.TIMER0.tasks_start.write(|w| unsafe { w.bits(1) });

            // *RADIO.borrow(cs).borrow_mut() = Some(Radio::new(p.RADIO));
            *TIMER.borrow(cs).borrow_mut() = Some(p.TIMER0);
            *RTC.borrow(cs).borrow_mut() = Some(p.RTC0);
            *DISPLAY.borrow(cs).borrow_mut() = Some(display);
            *STATE.borrow(cs).borrow_mut() = Some(0);
            *TX.borrow(cs).borrow_mut() = Some(serial_tx);
        });

        if let Some(mut p) = cortex_m::Peripherals::take() {
            p.NVIC.enable(nrf51::Interrupt::RTC0);
            nrf51::NVIC::unpend(nrf51::Interrupt::RTC0);
            p.NVIC.enable(nrf51::Interrupt::TIMER0);
            nrf51::NVIC::unpend(nrf51::Interrupt::TIMER0);
            /*
            p.NVIC.enable(nrf51::Interrupt::RADIO);
            nrf51::NVIC::unpend(nrf51::Interrupt::RADIO);
            */
        }
    }
    loop {}
}

// interrupt!(RADIO, radio_event);
interrupt!(TIMER0, timer0_event);
interrupt!(RTC0, rtc0_event);

fn rtc0_event() {
    cortex_m::interrupt::free(|cs| {
        if let (Some(s), Some(d), Some(r)) = (
            STATE.borrow(cs).borrow_mut().deref_mut(),
            DISPLAY.borrow(cs).borrow_mut().deref_mut(),
            RTC.borrow(cs).borrow_mut().deref_mut()) {
            r.events_tick.reset();
            *s = match *s {
                0 => {
                    d.display(MID_DOTT);
                    1
                }
                1 => {
                    d.display(LITTLE_HEART);
                    2
                }
                2 => {
                    d.display(HEART);
                    3
                }
                3 => {
                    d.display(LITTLE_HEART);
                    4
                }
                4 => {
                    d.display(MID_DOTT);
                    5
                }
                5 => {
                    d.display(CLEAR);
                    0
                }
                _ => {
                    0
                }
            }
        }
    });
}
/*
fn radio_event() {
    cortex_m::interrupt::free(|cs| {

    });
}
*/
fn timer0_event() {
    cortex_m::interrupt::free(|cs| {
        if let (Some(t), Some(d)) = (
            TIMER.borrow(cs).borrow_mut().deref_mut(),
            DISPLAY.borrow(cs).borrow_mut().deref_mut()) {
            t.events_compare[0].reset();
            t.cc[0].write(|w| unsafe { w.bits(2000) });
            t.tasks_start.write(|w| unsafe { w.bits(1) });
            d.update_row();
        }
    });
}
