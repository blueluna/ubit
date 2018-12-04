//! # The micro:bit
//! 
//! Wrappers for BBC micro:bit functionality

#![no_std]

extern crate nrf51;
pub extern crate nrf51_hal as hal;
pub extern crate byteorder;

pub mod radio;
pub mod leds;
pub mod datagram;
pub mod package;
