use core::convert::From;
use core::str;

use byteorder::{ByteOrder, BigEndian};

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

pub struct PackageHeader
{
    package_type: PackageType,
    time: i32,
    serial_number: i32,
}

impl PackageHeader {
    pub fn unpack(buffer: &[u8]) -> PackageHeader {
        assert!(buffer.len() > 8);
        let package_type = PackageType::from(buffer[0]);
        let mut time = 0i32;
        let mut serial_number = 0i32;
        if package_type != PackageType::Unknown {
            time = BigEndian::read_i32(&buffer[1..4]);
            serial_number = BigEndian::read_i32(&buffer[5..8]);
        }
        PackageHeader {
            package_type,
            time,
            serial_number,
        }
    }

    pub fn package_type(&self) -> PackageType {
        self.package_type.clone()
    }

    pub fn time(&self) -> i32 {
        self.time
    }

    pub fn serial_number(&self) -> i32 {
        self.serial_number
    }
}

pub enum Package
{
    Integer(PackageHeader, i32),
    IntegerValue(PackageHeader, i32),
    Other(PackageHeader),
    Unknown,
}

impl Package {
    pub fn unpack(buffer: &[u8]) -> Package {
        if buffer.len() < 9 {
            return Package::Unknown;
        }
        let ph = PackageHeader::unpack(&buffer[0..]);
        match ph.package_type {
            PackageType::Integer => {
                if buffer.len() > 12 {
                    let value = BigEndian::read_i32(&buffer[9..12]);
                    return Package::Integer(ph, value);
                }
                return Package::Unknown;
            }
            PackageType::IntegerValue => {
                if buffer.len() > 12 {
                    let value = BigEndian::read_i32(&buffer[9..12]);
                    return Package::IntegerValue(ph, value);
                }
                return Package::Unknown;
            }
            PackageType::Unknown => {
                return Package::Unknown;
            }
            _ => {
                return Package::Other(ph);
            }
        }
    }
}