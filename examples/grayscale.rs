#![no_std]
#![no_main]

extern crate panic_semihosting;
extern crate cortex_m;
extern crate cortex_m_rt;
#[macro_use]
extern crate microbit;
extern crate ubit;

use core::cell::RefCell;
use core::fmt::Write;
use core::ops::DerefMut;

use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;

use ubit::hal::prelude::*;
use ubit::hal::serial;
use ubit::hal::serial::BAUD115200;

use ubit::leds;

static TIMER: Mutex<RefCell<Option<microbit::TIMER0>>> = Mutex::new(RefCell::new(None));
static RTC: Mutex<RefCell<Option<microbit::RTC0>>> = Mutex::new(RefCell::new(None));
static DISPLAY: Mutex<RefCell<Option<leds::Display>>> = Mutex::new(RefCell::new(None));
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

            let display = leds::Display::new(
                col1, col2, col3, col4, col5, col6, col7, col8, col9, row1, row2, row3,
            );

            *DISPLAY.borrow(cs).borrow_mut() = Some(display);

            // Configure RX and TX pins accordingly
            let tx = gpio.pin24.into_push_pull_output().downgrade();
            let rx = gpio.pin25.into_floating_input().downgrade();
            let (mut serial_tx, _) = serial::Serial::uart0(p.UART0, tx, rx, BAUD115200).split();

            let _ = write!(serial_tx, "\n\rLet there be light\n\r");

            *TX.borrow(cs).borrow_mut() = Some(serial_tx);
            // Configure RTC with 125 ms resolution 
            p.RTC0.prescaler.write(|w| unsafe { w.bits(4095) });
            // Enable interrupt for tick
            p.RTC0.intenset.write(|w| w.tick().set_bit());
            // Start counter
            p.RTC0.tasks_start.write(|w| unsafe { w.bits(1) });
            *RTC.borrow(cs).borrow_mut() = Some(p.RTC0);

            // Configure a timer with 1us resolution
            p.TIMER0.bitmode.write(|w| w.bitmode()._32bit());
            p.TIMER0.prescaler.write(|w| unsafe { w.prescaler().bits(4) });
            p.TIMER0.intenset.write(|w| w.compare0().set());
            p.TIMER0.shorts.write(|w| w.compare0_clear().enabled()
                .compare0_stop().enabled());
            p.TIMER0.cc[0].write(|w| unsafe { w.bits(2000) });
            p.TIMER0.tasks_start.write(|w| unsafe { w.bits(1) });

            *TIMER.borrow(cs).borrow_mut() = Some(p.TIMER0);
        });

        if let Some(mut p) = cortex_m::Peripherals::take() {
            p.NVIC.enable(nrf51::Interrupt::RTC0);
            nrf51::NVIC::unpend(nrf51::Interrupt::RTC0);
            p.NVIC.enable(nrf51::Interrupt::TIMER0);
            nrf51::NVIC::unpend(nrf51::Interrupt::TIMER0);
        }
    }
    loop {}
}

interrupt!(TIMER0, timer0_event);
interrupt!(RTC0, rtc0_event);

fn rtc0_event() {
    cortex_m::interrupt::free(|cs| {
        if let (Some(display), Some(rtc)) = (
            DISPLAY.borrow(cs).borrow_mut().deref_mut(),
            RTC.borrow(cs).borrow_mut().deref_mut()) {
            rtc.events_tick.reset();
            let counter = rtc.counter.read().bits();
            let intensity = (counter & 0xff) as u8;
            let image = [
                [intensity, intensity, intensity, intensity, intensity],
                [intensity, intensity, intensity, intensity, intensity],
                [intensity, intensity, intensity, intensity, intensity],
                [intensity, intensity, intensity, intensity, intensity],
                [intensity, intensity, intensity, intensity, intensity],
            ];
            display.display(image);
        }
    });
}

fn timer0_event() {
    cortex_m::interrupt::free(|cs| {
        if let (Some(timer), Some(display)) = (
            TIMER.borrow(cs).borrow_mut().deref_mut(),
            DISPLAY.borrow(cs).borrow_mut().deref_mut())
        {
            timer.events_compare[0].reset();
            timer.cc[0].write(|w| unsafe { w.bits(2000) });
            timer.tasks_start.write(|w| unsafe { w.bits(1) });
            display.update_row();
        }
    });
}

