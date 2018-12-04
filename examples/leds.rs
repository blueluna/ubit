#![no_std]
#![no_main]

extern crate panic_semihosting;
extern crate cortex_m_rt;
#[macro_use]
extern crate microbit;

use core::sync::atomic::Ordering;
use core::sync::atomic::compiler_fence;

use core::cell::RefCell;
use core::fmt::Write;
use core::ops::DerefMut;

use cortex_m::interrupt::Mutex;
use cortex_m_rt::entry;

use ubit::hal::prelude::*;
use ubit::hal::serial;
use ubit::hal::gpio::{Floating, Input};
use ubit::hal::serial::BAUD115200;
use ubit::leds::images;
use ubit::radio;
use ubit::leds;
use ubit::package;

struct ProgramState {
    current_state: u32,
    next_state: u32,
    change_at: u32,
    rtc: microbit::RTC0,
}

impl ProgramState {
    pub fn new(rtc: microbit::RTC0) -> Self {
        // Configure RTC with 125 ms resolution 
        rtc.prescaler.write(|w| unsafe { w.bits(4095) });
        // Enable interrupt for tick
        rtc.intenset.write(|w| w.tick().set_bit());
        // Start counter
        rtc.tasks_start.write(|w| unsafe { w.bits(1) });
        ProgramState { current_state: 0, next_state: u32::max_value(), change_at: 10, rtc }
    }

    pub fn ticks(&self) -> u32 {
        self.rtc.counter.read().bits()
    }

    pub fn change_state(&mut self, state: u32, delay: Option<u32>)
    {
        self.next_state = state;
        self.change_at = self.ticks() + delay.unwrap_or(1);
    }

    pub fn rtc_interrupt(&mut self) -> Option<u32>
    {
        self.rtc.events_tick.reset();
        let now = self.ticks();
        if self.change_at > 0 && self.change_at <= now {
            self.current_state = self.next_state;
            self.change_at = 0;
            Some(self.current_state)
        }
        else {
            None
        }
    }
}

struct ButtonState {
    gpio_task_event: microbit::GPIOTE,
    button_a: microbit::hal::gpio::gpio::PIN17<Input<Floating>>,
    button_b: microbit::hal::gpio::gpio::PIN26<Input<Floating>>,
}

static RDIO: Mutex<RefCell<Option<radio::Radio>>> = Mutex::new(RefCell::new(None));
static TIMER: Mutex<RefCell<Option<microbit::TIMER0>>> = Mutex::new(RefCell::new(None));
static DISPLAY: Mutex<RefCell<Option<leds::Display>>> = Mutex::new(RefCell::new(None));
static STATE: Mutex<RefCell<Option<ProgramState>>> = Mutex::new(RefCell::new(None));
static TX: Mutex<RefCell<Option<serial::Tx<microbit::UART0>>>> = Mutex::new(RefCell::new(None));
static BTN: Mutex<RefCell<Option<ButtonState>>> = Mutex::new(RefCell::new(None));

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

            // Buttons
            let button_b = gpio.pin26.into_floating_input(); // B
            let button_a = gpio.pin17.into_floating_input(); // A
            
            /* Set up GPIO 17 (button A) to generate an interrupt when pulled down */
            p.GPIOTE.config[0]
                .write(|w| unsafe { w.mode().event().psel().bits(17).polarity().toggle() });
            p.GPIOTE.intenset.write(|w| w.in0().set_bit());
            p.GPIOTE.events_in[0].write(|w| unsafe { w.bits(0) });

            /* Set up GPIO 26 (button B) to generate an interrupt when pulled down */
            p.GPIOTE.config[1]
                .write(|w| unsafe { w.mode().event().psel().bits(26).polarity().toggle() });
            p.GPIOTE.intenset.write(|w| w.in1().set_bit());
            p.GPIOTE.events_in[1].write(|w| unsafe { w.bits(0) });

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

            // Configure RX and TX pins
            let tx = gpio.pin24.into_push_pull_output().downgrade();
            let rx = gpio.pin25.into_floating_input().downgrade();
            let (serial_tx, _) = serial::Serial::uart0(p.UART0, tx, rx, BAUD115200).split();

            *BTN.borrow(cs).borrow_mut() = Some(ButtonState {
                gpio_task_event: p.GPIOTE,
                button_a,
                button_b,
                });

            *TX.borrow(cs).borrow_mut() = Some(serial_tx);
            
            // Configure a timer with 1us resolution
            p.TIMER0.bitmode.write(|w| w.bitmode()._32bit());
            p.TIMER0.prescaler.write(|w| unsafe { w.prescaler().bits(4) });
            p.TIMER0.intenset.write(|w| w.compare0().set());
            p.TIMER0.shorts.write(|w| w.compare0_clear().enabled()
                .compare0_stop().enabled());
            p.TIMER0.cc[0].write(|w| unsafe { w.bits(2000) });
            p.TIMER0.tasks_start.write(|w| unsafe { w.bits(1) });

            let mut radio = radio::Radio::new(p.RADIO);
            radio.set_group(1);
            radio.start_receive();

            *RDIO.borrow(cs).borrow_mut() = Some(radio);
            *TIMER.borrow(cs).borrow_mut() = Some(p.TIMER0);
            *STATE.borrow(cs).borrow_mut() = Some(ProgramState::new(p.RTC0));
        });

        if let Some(mut p) = cortex_m::Peripherals::take() {
            p.NVIC.enable(nrf51::Interrupt::RTC0);
            nrf51::NVIC::unpend(nrf51::Interrupt::RTC0);
            p.NVIC.enable(nrf51::Interrupt::TIMER0);
            nrf51::NVIC::unpend(nrf51::Interrupt::TIMER0);
            p.NVIC.enable(nrf51::Interrupt::TIMER0);
            nrf51::NVIC::unpend(nrf51::Interrupt::TIMER0);
            p.NVIC.enable(microbit::Interrupt::GPIOTE);
            nrf51::NVIC::unpend(microbit::Interrupt::GPIOTE);
            p.NVIC.enable(nrf51::Interrupt::RADIO);
            nrf51::NVIC::unpend(nrf51::Interrupt::RADIO);
        }
    }
    loop {}
}

interrupt!(RADIO, radio_event);
interrupt!(TIMER0, timer0_event);
interrupt!(RTC0, rtc0_event);
interrupt!(GPIOTE, gpiote_event);

fn rtc0_event() {
    compiler_fence(Ordering::AcqRel);
    cortex_m::interrupt::free(|cs| {
        if let (Some(state), Some(display)) = (
            STATE.borrow(cs).borrow_mut().deref_mut(),
            DISPLAY.borrow(cs).borrow_mut().deref_mut())
        {
            let current_state = state.rtc_interrupt();
            if current_state.is_none() { return; }
            let (next_state, next_delay) = match current_state.unwrap() {
                0 => {
                    display.display(images::MID_DOT);
                    (1, 1)
                }
                1 => {
                    display.display(images::LITTLE_HEART);
                    (2, 1)
                }
                2 => {
                    display.display(images::HEART);
                    (3, 1)
                }
                3 => {
                    display.display(images::LITTLE_HEART);
                    (4, 1)
                }
                4 => {
                    display.display(images::MID_DOT);
                    (5, 1)
                }
                5 => {
                    display.display(images::CLEAR);
                    (0, 1)
                }
                100 => {
                    display.display(images::HAPPY);
                    (0, 10)
                }
                101 => {
                    display.display(images::SAD);
                    (0, 10)
                }
                102 => {
                    display.display(images::GHOST);
                    (0, 10)
                }
                200 => {
                    display.display(images::HAPPY);
                    (0, 10)
                }
                201 => {
                    display.display(images::SAD);
                    (0, 10)
                }
                202 => {
                    display.display(images::GHOST);
                    (0, 10)
                }
                _ => {
                    (0, 1)
                }
            };
            state.change_state(next_state, Some(next_delay));
        }
    });
}

fn radio_event() {
    compiler_fence(Ordering::AcqRel);
    cortex_m::interrupt::free(|cs| {
        if let (Some(radio), Some(tx), Some(state)) = (
            RDIO.borrow(cs).borrow_mut().deref_mut(),
            TX.borrow(cs).borrow_mut().deref_mut(),
            STATE.borrow(cs).borrow_mut().deref_mut())
        {
            let mut data = [0; radio::MAX_PACKAGE_SIZE];
            let _ = radio.receive(&mut data);
            let p = package::Package::unpack(&data[..]);
            if p.header.datagram_header.length() >= 16 {
                match p.data {
                    package::PackageData::Integer(value) => {
                        if value >= 0 && value < 3 {
                            state.change_state(200 + (value as u32), None);
                        }
                    }
                    package::PackageData::IntegerValue(value) => {
                        if value >= 0 && value < 3 {
                            state.change_state(200 + (value as u32), None);
                        }
                    }
                    package::PackageData::Other => {
                        write!(tx, "Other Package\n\r");
                    }
                    package::PackageData::Unknown => {
                        write!(tx, "Unknown Package\n\r");
                    }
                }
            }
            radio.start_receive();
        }
    });
}

fn timer0_event() {
    compiler_fence(Ordering::AcqRel);
    cortex_m::interrupt::free(|cs| {
        if let (Some(timer), Some(display)) = (
            TIMER.borrow(cs).borrow_mut().deref_mut(),
            DISPLAY.borrow(cs).borrow_mut().deref_mut())
        {
            timer.events_compare[0].reset();
            let mut delay = 0;
            while delay == 0 {
                delay = display.update_col();
            }
            timer.cc[0].write(|w| unsafe { w.bits(delay) });
            timer.tasks_start.write(|w| unsafe { w.bits(1) });
        }
    });
}

fn gpiote_event() {
    compiler_fence(Ordering::AcqRel);
    cortex_m::interrupt::free(|cs| {
        if let (Some(btn), Some(state)) = (
            BTN.borrow(cs).borrow_mut().deref_mut(),
            STATE.borrow(cs).borrow_mut().deref_mut())
        {
            match (btn.button_a.is_low(), btn.button_b.is_low()) {
                (false, false) => (),
                (true, false) => { state.change_state(100, None); }
                (false, true) => { state.change_state(101, None); }
                (true, true) => { state.change_state(102, None); }
            }
            /* Clear events */
            btn.gpio_task_event.events_in[0].write(|w| unsafe { w.bits(0) });
            btn.gpio_task_event.events_in[1].write(|w| unsafe { w.bits(0) });
        }
    });
}

