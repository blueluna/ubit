use nrf51::RADIO;
use nrf51::radio::state::STATER;

pub const BASE_ADDRESS: u32 = 0x75626974;
pub const DEFAULT_GROUP: u8 = 0;
pub const MAX_PACKET_SIZE: usize = 32;
// pub const HEADER_SIZE: usize = 4;
// pub const MAXIMUM_RX_BUFFERS: usize = 4;
pub const CRC_POLY: u32 = 0x00011021;
pub const CRC_PRESET: u32 = 0x0000ffff;
pub const WHITENING_IV: u8 = 0x18;

pub type PacketBuffer = [u8; MAX_PACKET_SIZE];

pub struct Radio {
    radio: RADIO,
    rx_buf: PacketBuffer,
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
                .maxlen().bits(MAX_PACKET_SIZE as u8)
                .balen().bits(4)
                .whiteen().set_bit()
            );
            // Initialise 16-bit CRC
            radio.crccnf.write(|w| w.len().two());
            radio.crcinit.write(|w| w.bits(CRC_PRESET));
            radio.crcpoly.write(|w| w.crcpoly().bits(CRC_POLY & 0x0000ffff));
            // Configure base address
            radio.base0.write(|w| w.bits(BASE_ADDRESS << 8));
            radio.prefix0.write(|w| w.ap0().bits(DEFAULT_GROUP));

            radio.datawhiteiv.write(|w|
                w.datawhiteiv().bits(WHITENING_IV));
        }

        radio.shorts.write(|w| w
            .ready_start().enabled()
            .end_disable().enabled()
        );

        Self {
            radio,
            rx_buf: [0u8; MAX_PACKET_SIZE],
        }
    }

    /// Returns the current radio state.
    pub fn state(&self) -> STATER {
        self.radio.state.read().state()
    }

    pub fn start_receive(&mut self)
    {
        let rx_buf = &mut self.rx_buf as *mut _ as u32;
        self.radio.packetptr.write(|w| unsafe { w.bits(rx_buf) });
        self.radio.rxaddresses.write(|w| w.addr1().enabled());
    }
}
