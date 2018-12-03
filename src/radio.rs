use core::convert::From;
use core::sync::atomic::Ordering;
use core::sync::atomic::compiler_fence;

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

#[derive(PartialEq)]
pub enum PacketType {
    Integer,
    IntegerValue,
    String,
    Buffer,
    Double,
    DoubleValue,
    Unknown,
}

impl From<u8> for PacketType {
    fn from(value: u8) -> PacketType {
        match value {
            0 => PacketType::Integer,
            1 => PacketType::IntegerValue,
            2 => PacketType::String,
            3 => PacketType::Buffer,
            4 => PacketType::Double,
            5 => PacketType::DoubleValue,
            _ => PacketType::Unknown,
        }
    }
}

pub struct PacketHeader
{
    packet_type: PacketType,
    time: i32,
    serial_number: i32,
}

impl PacketHeader {
    pub fn unpack(buffer: &[u8]) -> PacketHeader {
        assert!(buffer.len() > 8);
        let packet_type = PacketType::from(buffer[0]);
        let mut time = 0i32;
        let mut serial_number = 0i32;
        if packet_type != PacketType::Unknown {
            time = (buffer[1] as i32) << 24 | (buffer[2] as i32) << 16
                | (buffer[3] as i32) << 8 | (buffer[4] as i32);
            serial_number = (buffer[5] as i32) << 24 | (buffer[6] as i32) << 16
                | (buffer[7] as i32) << 8 | (buffer[8] as i32);
        }
        PacketHeader {
            packet_type,
            time,
            serial_number,
        }
    }
}

pub enum Packet
{
    Integer(PacketHeader, i32),
    IntegerValue(PacketHeader, i32),
    Other(PacketHeader),
    Unknown,
}

impl Packet {
    pub fn unpack(buffer: &[u8]) -> Packet {
        if buffer.len() < 9 {
            return Packet::Unknown;
        }
        let ph = PacketHeader::unpack(&buffer[0..]);
        match ph.packet_type {
            PacketType::Integer => {
                if buffer.len() > 12 {
                    let value = (buffer[9] as i32) << 24
                        | (buffer[10] as i32) << 16
                        | (buffer[11] as i32) << 8
                        | (buffer[12] as i32);
                    return Packet::Integer(ph, value);
                }
                return Packet::Unknown;
            }
            PacketType::IntegerValue => {
                if buffer.len() > 12 {
                    let value = (buffer[9] as i32) << 24
                        | (buffer[10] as i32) << 16
                        | (buffer[11] as i32) << 8
                        | (buffer[12] as i32);
                    return Packet::IntegerValue(ph, value);
                }
                return Packet::Unknown;
            }
            PacketType::Unknown => {
                return Packet::Unknown;
            }
            _ => {
                return Packet::Other(ph);
            }
        }
    }
}

/// # The micro:bit radio
/// 
/// The goal is to be able to communicate with software written with MakeCode
/// or similar.
/// 
/// The package format seems to be the following,
/// 
/// ```notrust
/// Packet Spec:
/// | 0           | 1 ... 4     | 5 ... 8       | 9 ... 28
/// ------------------------------------------------------
/// | packet type | system time | serial number | payload
/// ```
/// 
/// The radio is configured as nrf24 1 mbit....
/// 
/// ## Reference
/// 
/// * <https://github.com/lancaster-university/microbit-dal/blob/master/source/drivers/MicroBitRadio.cpp>
/// * <https://github.com/Microsoft/pxt-microbit/blob/master/libs/radio/radio.cpp>
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
            rx_buf: [0u8; MAX_PACKET_SIZE],
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

    pub fn receive(&mut self, dst: &mut PacketBuffer) -> usize
    {
        compiler_fence(Ordering::AcqRel);
        self.radio.events_end.reset();
        if self.radio.crcstatus.read().crcstatus().is_crcok() {
            let length = self.rx_buf[0];
            if length > 0 {
                dst.copy_from_slice(&self.rx_buf[..MAX_PACKET_SIZE]);
                return length as usize
            }
        }
        0
    }
}
