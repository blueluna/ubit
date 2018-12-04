//! Micro:bit datagram format

use core::convert::From;

#[derive(Clone, PartialEq)]
pub enum DatagramProtocol {
    Datagram,
    EventBus,
    Unknown,
}

impl From<u8> for DatagramProtocol {
    fn from(value: u8) -> DatagramProtocol {
        match value {
            1 => DatagramProtocol::Datagram,
            2 => DatagramProtocol::EventBus,
            _ => DatagramProtocol::Unknown,
        }
    }
}

/// # Datagram Header
///
/// ```notrust
/// | 0      | 1       | 2     | 3        | 4 ...
/// ----------------------------------------------
/// | length | version | group | protocol | payload
/// ```
/// Datagram length is length of package without the length itselfe
/// Protocol is either
///  * 0, Datagram
///  * 1, EventBus
///
pub struct DatagramHeader
{
    length: u8,
    version: u8,
    group: u8,
    protocol: DatagramProtocol,
}

impl DatagramHeader {
    /// Unpack a PackageHeader from the byte slice
    pub fn unpack(buffer: &[u8]) -> DatagramHeader {
        assert!(buffer.len() >= 4);
        let length = buffer[0];
        if length >= 3 {
            DatagramHeader {
                length,
                version: buffer[1],
                group: buffer[2],
                protocol: DatagramProtocol::from(buffer[3]),
            }
        }
        else {
            DatagramHeader {
                length,
                version: 0,
                group: 0,
                protocol: DatagramProtocol::Unknown,
            }
        }
    }
    /// Get the package length
    pub fn length(&self) -> u8 {
        self.length
    }
    /// Get the package payload length
    pub fn payload_length(&self) -> usize {
        if self.length > 3 { usize::from(self.length - 3) } else { 0 }
    }
    /// Get the package version
    pub fn version(&self) -> u8 {
        self.version
    }
    /// Get the package group
    pub fn group(&self) -> u8 {
        self.group
    }
    /// Get the package protocol
    pub fn protocol(&self) -> DatagramProtocol {
        self.protocol.clone()
    }
}
