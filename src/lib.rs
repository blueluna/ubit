//! # The micro:bit radio
//! 
//! The goal is to be able to communicate with software written with MakeCode
//! or similar.
//! 
//! The package format seems to be the following,
//! 
//! ```notrust
//! Packet Spec:
//! | 0              | 1 ... 4       | 5 ... 8           | 9 ... 28
//! ----------------------------------------------------------------
//! | packet type    | system time   | serial number     | payload
//! ```
//! 
//! The radio is configured as nrf24 1 mbit....
//! 
//! ## Reference
//! 
//! * <https://github.com/lancaster-university/microbit-dal/blob/master/source/drivers/MicroBitRadio.cpp>
//! * <https://github.com/Microsoft/pxt-microbit/blob/master/libs/radio/radio.cpp>


#![no_std]

extern crate nrf51;

mod radio;

pub use radio::Radio;
