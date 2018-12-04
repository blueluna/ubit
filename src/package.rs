//! MakeCode package format

use core::convert::From;

use byteorder::{ByteOrder, LittleEndian};

use crate::datagram::{DatagramHeader, DatagramProtocol};

#[derive(Clone, PartialEq)]
pub enum PackageType {
    Integer,
    IntegerValue,
    String,
    Buffer,
    Double,
    DoubleValue,
    Unknown,
}

impl From<u8> for PackageType {
    fn from(value: u8) -> PackageType {
        match value {
            0 => PackageType::Integer,
            1 => PackageType::IntegerValue,
            2 => PackageType::String,
            3 => PackageType::Buffer,
            4 => PackageType::Double,
            5 => PackageType::DoubleValue,
            _ => PackageType::Unknown,
        }
    }
}

impl From<PackageType> for u8 {
    fn from(value: PackageType) -> u8 {
        match value {
            PackageType::Integer => 0,
            PackageType::IntegerValue => 1,
            PackageType::String => 2,
            PackageType::Buffer => 3,
            PackageType::Double => 4,
            PackageType::DoubleValue => 5,
            PackageType::Unknown => 0xff,
        }
    }
}

/// # Package Header
///
/// The goal is to be able to communicate with software written with MakeCode
/// or similar.
///
/// ```notrust
/// | 0    | 1 ... 4 | 5 ... 8       | 9 ... 28
/// ----------------------------------------------
/// | type | time    | serial number | payload
/// ```
/// Package type is either,
///  * 0, Integer value
///  * 1, Named integer value
///  * 2, String
///  * 3, Buffer
///  * 4, Double value
///  * 5, Named double value
/// 
/// ## Reference
///
/// * <https://github.com/Microsoft/pxt-microbit/blob/master/libs/radio/radio.cpp>
pub struct PackageHeader
{
    pub datagram_header: DatagramHeader,
    package_type: PackageType,
    time: u32,
    serial_number: u32,
}

impl PackageHeader {
    /// Unpack a PackageHeader from the byte slice
    pub fn unpack(buffer: &[u8]) -> PackageHeader {
        let datagram_header = DatagramHeader::unpack(&buffer[..]);
        let slice = &buffer[4..];
        let package_type =
            if datagram_header.protocol() != DatagramProtocol::Datagram
                || datagram_header.payload_length() <= 8
        {
            PackageType::Unknown
        }
        else {
            PackageType::from(slice[0])
        };
        if package_type != PackageType::Unknown {
            PackageHeader {
                datagram_header,
                package_type,
                time: LittleEndian::read_u32(&slice[1..=4]),
                serial_number: LittleEndian::read_u32(&slice[5..=8]),
            }
        }
        else {
            PackageHeader {
                datagram_header,
                package_type: PackageType::Unknown,
                time: 0,
                serial_number: 0,
            }
        }
    }

    /// Get the package type
    pub fn package_type(&self) -> PackageType {
        self.package_type.clone()
    }
    /// Get the package time
    pub fn time(&self) -> u32 {
        self.time
    }
    /// Get the package serial number
    pub fn serial_number(&self) -> u32 {
        self.serial_number
    }
    /// Get the package payload length
    pub fn payload_length(&self) -> usize {
        let length = self.datagram_header.payload_length();
        if length > 9 { length - 9 } else { 0 }
    }
}

/// # PackageData
/// 
pub enum PackageData
{
    Integer(i32),
    IntegerValue(i32),
    Other,
    Unknown,
}

pub struct Package {
    pub header: PackageHeader,
    pub data: PackageData,
}

impl Package {
    /// Unpack a Package from the byte slice
    pub fn unpack(buffer: &[u8]) -> Package {
        let header = PackageHeader::unpack(&buffer[..]);
        match header.package_type {
            PackageType::Integer => {
                if header.payload_length() >= 4 {
                    let value = LittleEndian::read_i32(&buffer[13..=16]);
                    return Package {
                        header,
                        data: PackageData::Integer(value),
                    };
                }
            }
            PackageType::IntegerValue => {
                if header.payload_length() >= 5 {
                    let value = LittleEndian::read_i32(&buffer[13..=16]);
                    return Package {
                        header,
                        data: PackageData::IntegerValue(value),
                    };
                }
            }
            PackageType::Unknown => (),
            _ => {
                return Package {
                    header,
                    data: PackageData::Other,
                };
            }
        }
        Package {
            header,
            data: PackageData::Unknown,
        }
    }
}