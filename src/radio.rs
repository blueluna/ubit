//! NRF Radio

use core::sync::atomic::Ordering;
use core::sync::atomic::compiler_fence;

use nrf51::RADIO;
use nrf51::radio::state::STATER;

pub const BASE_ADDRESS: u32 = 0x75626974;
pub const DEFAULT_GROUP: u8 = 0;
pub const MAX_PACKAGE_SIZE: usize = 32;
// pub const HEADER_SIZE: usize = 4;
// pub const MAXIMUM_RX_BUFFERS: usize = 4;
pub const CRC_POLY: u32 = 0x00011021;
pub const CRC_PRESET: u32 = 0x0000ffff;
pub const WHITENING_IV: u8 = 0x18;

pub type PackageBuffer = [u8; MAX_PACKAGE_SIZE];


/// # The micro:bit radio
/// 
/// The goal is to be able to communicate with software written with MakeCode
/// or similar.
/// 
/// The radio is configured as Nordic properitary 1 Mbit radio, 16-bit CRC.
/// 
/// ## Reference
/// 
/// * <https://github.com/lancaster-university/microbit-dal/blob/master/source/drivers/MicroBitRadio.cpp>
pub struct Radio {
    radio: RADIO,
    rx_buf: PackageBuffer,
}

impl Radio {
    pub fn new(radio: RADIO) -> Self {
        assert!(radio.state.read().state().is_disabled());

        radio.mode.write(|w| w.mode().nrf_1mbit());
        radio.txpower.write(|w| w.txpower().pos4d_bm());

        unsafe {
            // On-air package length field size, 8-bits
            radio.pcnf0.write(|w| w.lflen().bits(8));
            // Configure maximum package size
            // Base address length, 5
            // Enable whitening
            radio.pcnf1.write(|w| w
                .maxlen().bits(MAX_PACKAGE_SIZE as u8)
                .balen().bits(4)
                .whiteen().set_bit()
            );
            // Initialise 16-bit CRC
            radio.crccnf.write(|w| w.len().two());
            radio.crcinit.write(|w| w.bits(CRC_PRESET));
            radio.crcpoly.write(|w| w.crcpoly().bits(CRC_POLY & 0x0000ffff));
            // Configure base address
            radio.base0.write(|w| w.bits(BASE_ADDRESS));
            radio.prefix0.write(|w| w.ap0().bits(DEFAULT_GROUP));
            radio.frequency.write(|w| w.frequency().bits(7u8));

            radio.datawhiteiv.write(|w|
                w.datawhiteiv().bits(WHITENING_IV));
        }

        radio.shorts.write(|w| w
            .ready_start().enabled()
            .end_disable().enabled()
        );

        Self {
            radio,
            rx_buf: [0u8; MAX_PACKAGE_SIZE],
        }
    }

    /// Returns the current radio state.
    pub fn state(&self) -> STATER {
        self.radio.state.read().state()
    }

    /// Change the group
    pub fn set_group(&mut self, group: u8)
    {
        self.radio.prefix0.write(|w| unsafe { w.ap0().bits(group) });
    }

    pub fn start_receive(&mut self)
    {
        compiler_fence(Ordering::AcqRel);
        let rx_buf = &mut self.rx_buf as *mut _ as u32;
        self.radio.packetptr.write(|w| unsafe { w.bits(rx_buf) });
        self.radio.rxaddresses.write(|w| w.addr0().enabled());
        self.radio.intenset.write(|w| w.end().set());
        self.radio.tasks_rxen.write(|w| unsafe { w.bits(1) });
    }

    pub fn receive(&mut self, dst: &mut PackageBuffer) -> usize
    {
        compiler_fence(Ordering::AcqRel);
        self.radio.events_end.reset();
        if self.radio.crcstatus.read().crcstatus().is_crcok() {
            let length = self.rx_buf[0];
            if length > 0 {
                dst.copy_from_slice(&self.rx_buf[..MAX_PACKAGE_SIZE]);
                return length as usize
            }
        }
        0
    }
}
