use nrf51::{FICR, RADIO};
use nrf51::radio::state::STATER;

use core::time::Duration;

pub const MAX_PACKAGE_SIZE: usize = 64;

pub type TransmitBuffer = [u8; MAX_PACKAGE_SIZE];

pub struct Radio {
    radio: RADIO,
    rx_buf: &'static mut TransmitBuffer,
    tx_buf: &'static mut TransmitBuffer,
}

impl Radio {
    pub fn new(radio: RADIO, ficr: &FICR, tx_buf: &'static mut TransmitBuffer) -> Self {
        assert!(radio.state.read().state().is_disabled());

        radio.mode.write(|w| w.mode().ble_1mbit());
        radio.txpower.write(|w| w.txpower().pos4d_bm());

        unsafe {
            radio.pcnf1.write(|w| w
                .maxlen().bits(MAX_PACKAGE_SIZE as u8)   // no packet length limit
                .balen().bits(3)     // 3-Byte Base Address + 1-Byte Address Prefix
                .whiteen().set_bit() // Enable Data Whitening over PDU+CRC
            );
            radio.crccnf.write(|w| w
                .skipaddr().set_bit()   // skip address since only the S0, Length, S1 and Payload need CRC
                .len().three()          // 3 Bytes = CRC24
            );
            radio.crcpoly.write(|w| w.crcpoly().bits(CRC_POLY & 0xFFFFFF));

            radio.base0.write(|w| w.bits(ADVERTISING_ADDRESS << 8));
            radio.prefix0.write(|w| w.ap0().bits((ADVERTISING_ADDRESS >> 24) as u8));
        }

        unsafe {
            radio.tifs.write(|w| w.tifs().bits(150));
        }

        // Configure shortcuts to simplify and speed up sending and receiving packets.
        radio.shorts.write(|w| w
            .ready_start().enabled()    // start transmission/recv immediately after ramp-up
            .end_disable().enabled()    // disable radio when transmission/recv is done
        );
        // We can now start the TXEN/RXEN tasks and the radio will do the rest and return to the
        // disabled state.

        Self {
            radio,
            tx_buf,
        }
    }

    /// Returns the current radio state.
    pub fn state(&self) -> STATER {
        self.radio.state.read().state()
    }
}
