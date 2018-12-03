#![no_std]
#![no_main]

extern crate panic_halt;
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
use ubit::hal::gpio::{Floating, Input};
use ubit::hal::serial::BAUD115200;
use ubit::leds::images;

use ubit::radio;
use ubit::leds;

struct ProgramState {
    state: u32,
    tick: u32
}

struct ButtonState {
    gpio_task_event: microbit::GPIOTE,
    button_a: microbit::hal::gpio::gpio::PIN17<Input<Floating>>,
    button_b: microbit::hal::gpio::gpio::PIN26<Input<Floating>>,
}

static RDIO: Mutex<RefCell<Option<radio::Radio>>> = Mutex::new(RefCell::new(None));
static TIMER: Mutex<RefCell<Option<microbit::TIMER0>>> = Mutex::new(RefCell::new(None));
static RTC: Mutex<RefCell<Option<microbit::RTC0>>> = Mutex::new(RefCell::new(None));
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

            // Configure RX and TX pins accordingly
            let tx = gpio.pin24.into_push_pull_output().downgrade();
            let rx = gpio.pin25.into_floating_input().downgrade();
            let (mut serial_tx, _) = serial::Serial::uart0(p.UART0, tx, rx, BAUD115200).split();

            let _ = write!(serial_tx, "\n\rStarting!\n\r");

            *BTN.borrow(cs).borrow_mut() = Some(ButtonState {
                gpio_task_event: p.GPIOTE,
                button_a,
                button_b,
                });

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

            let mut radio = radio::Radio::new(p.RADIO);
            radio.set_group(1);
            radio.start_receive();

            *RDIO.borrow(cs).borrow_mut() = Some(radio);
            *TIMER.borrow(cs).borrow_mut() = Some(p.TIMER0);
            *STATE.borrow(cs).borrow_mut() = Some(ProgramState { state: 0, tick: 0 });
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
    cortex_m::interrupt::free(|cs| {
        if let (Some(s), Some(d), Some(r)) = (
            STATE.borrow(cs).borrow_mut().deref_mut(),
            DISPLAY.borrow(cs).borrow_mut().deref_mut(),
            RTC.borrow(cs).borrow_mut().deref_mut()) {
            r.events_tick.reset();
            let counter = r.counter.read().bits();
            if counter >= s.tick {
                s.tick = counter + 1;
                s.state = match s.state {
                    0 => {
                        d.display(images::MID_DOT);
                        1
                    }
                    1 => {
                        d.display(images::LITTLE_HEART);
                        2
                    }
                    2 => {
                        d.display(images::HEART);
                        3
                    }
                    3 => {
                        d.display(images::LITTLE_HEART);
                        4
                    }
                    4 => {
                        d.display(images::MID_DOT);
                        5
                    }
                    5 => {
                        d.display(images::CLEAR);
                        0
                    }
                    100 => {
                        d.display(images::HAPPY);
                        s.tick = counter + 10;
                        0
                    }
                    101 => {
                        d.display(images::SAD);
                        s.tick = counter + 10;
                        0
                    }
                    102 => {
                        d.display(images::GHOST);
                        s.tick = counter + 10;
                        0
                    }
                    200 => {
                        d.display(images::WAVE);
                        s.tick = counter + 10;
                        0
                    }
                    _ => {
                        0
                    }
                }
            }
        }
    });
}

fn radio_event() {
    cortex_m::interrupt::free(|cs| {
        if let (Some(s), Some(r), Some(rtc)) = (
            STATE.borrow(cs).borrow_mut().deref_mut(),
            RDIO.borrow(cs).borrow_mut().deref_mut(),
            RTC.borrow(cs).borrow_mut().deref_mut())
        {
            let counter = rtc.counter.read().bits();
            let mut packet = [0; radio::MAX_PACKET_SIZE];
            if r.receive(&mut packet) > 0 {
                let rt = radio::PacketType::from(packet[1]);
                if rt != radio::PacketType::None {
                    s.state = 200; s.tick = counter;
                }
            }
            r.start_receive();
        }
    });
}

fn timer0_event() {
    cortex_m::interrupt::free(|cs| {
        if let (Some(t), Some(d)) = (
            TIMER.borrow(cs).borrow_mut().deref_mut(),
            DISPLAY.borrow(cs).borrow_mut().deref_mut())
        {
            t.events_compare[0].reset();
            t.cc[0].write(|w| unsafe { w.bits(2000) });
            t.tasks_start.write(|w| unsafe { w.bits(1) });
            d.update_row();
        }
    });
}

fn gpiote_event() {
    cortex_m::interrupt::free(|cs| {
        if let (Some(b), Some(s), Some(r)) = (
            BTN.borrow(cs).borrow_mut().deref_mut(),
            STATE.borrow(cs).borrow_mut().deref_mut(),
            RTC.borrow(cs).borrow_mut().deref_mut()) {
            let counter = r.counter.read().bits();
            match (b.button_a.is_low(), b.button_b.is_low()) {
                (false, false) => (),
                (true, false) => { s.state = 100; s.tick = counter; }
                (false, true) => { s.state = 101; s.tick = counter; }
                (true, true) => { s.state = 102; s.tick = counter; }
            }
            /* Clear events */
            b.gpio_task_event.events_in[0].write(|w| unsafe { w.bits(0) });
            b.gpio_task_event.events_in[1].write(|w| unsafe { w.bits(0) });
        }
    });
}

