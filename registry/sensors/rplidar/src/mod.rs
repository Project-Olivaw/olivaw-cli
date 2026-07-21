//! Pure RPLIDAR protocol encoding and decoding.
//!
//! Bytes in, typed values out. This module performs **no I/O**, depends only
//! on `core`, and is fully unit-testable with no hardware attached. Feed it
//! bytes from any transport — a serial port, a UART driver, a recording.
//!
//! Wire format reference: the SLAMTEC RPLIDAR public protocol specification.
//! Every request starts with the sync byte [`SYNC_BYTE`]; every response is
//! preceded by a 7-byte descriptor.
//!
//! Vendored from `olivaw-lidar` (<https://github.com/Project-Olivaw/olivaw-lidar>),
//! whose `transport`/`device` layers show how to drive this over a serial port.

pub mod command;
pub mod descriptor;
pub mod info;
pub mod scan_node;

pub use command::{Command, MAX_REQUEST_LEN, MOTOR_PWM_MAX, SYNC_BYTE};

/// A protocol-level decoding failure.
///
/// These errors indicate that bytes received from the device do not match
/// the RPLIDAR wire format — typically a desynchronized stream, a corrupted
/// transfer, or an unexpected response to a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProtocolError {
    /// A response descriptor did not start with the `0xA5 0x5A` sync bytes.
    ///
    /// The stream is desynchronized; flush the input and retry the request.
    BadSync {
        /// The two bytes actually received where the sync bytes were expected.
        actual: [u8; 2],
    },

    /// A response descriptor used a reserved send-mode value (`0x2`/`0x3`).
    ReservedSendMode(u8),

    /// The response descriptor announced a different data type than the one
    /// the request expects.
    WrongDataType {
        /// Data type required by the request that was sent.
        expected: u8,
        /// Data type announced by the device.
        actual: u8,
    },

    /// The response descriptor announced a different payload length than the
    /// one the request expects.
    WrongLength {
        /// Payload length in bytes required by the request that was sent.
        expected: u32,
        /// Payload length announced by the device.
        actual: u32,
    },

    /// The response descriptor announced a different send mode (single vs.
    /// multi) than the one the request expects.
    WrongSendMode,

    /// A `GET_HEALTH` response carried a status byte outside `0..=2`.
    InvalidHealthStatus(u8),

    /// A 5-byte scan node failed its validity check (start-flag pair or
    /// check bit). The stream is desynchronized; discard one byte and retry.
    InvalidScanNode {
        /// First node byte: quality + start-flag pair.
        byte0: u8,
        /// Second node byte: low angle bits + check bit.
        byte1: u8,
    },

    /// A checksum-protected payload failed verification.
    ///
    /// Reserved for express-scan responses; standard responses carry no
    /// checksum.
    Checksum {
        /// Checksum computed over the received bytes.
        expected: u8,
        /// Checksum byte actually received.
        actual: u8,
    },
}

impl core::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadSync { actual } => {
                write!(f, "bad descriptor sync: expected [0xa5, 0x5a], got {actual:02x?}")
            }
            Self::ReservedSendMode(mode) => write!(f, "reserved response send mode {mode:#04x}"),
            Self::WrongDataType { expected, actual } => write!(
                f,
                "unexpected response data type: expected {expected:#04x}, got {actual:#04x}"
            ),
            Self::WrongLength { expected, actual } => write!(
                f,
                "unexpected response length: expected {expected}, got {actual}"
            ),
            Self::WrongSendMode => write!(f, "unexpected response send mode"),
            Self::InvalidHealthStatus(status) => {
                write!(f, "invalid health status byte {status:#04x}")
            }
            Self::InvalidScanNode { byte0, byte1 } => write!(
                f,
                "invalid scan node: byte0={byte0:#04x}, byte1={byte1:#04x}"
            ),
            Self::Checksum { expected, actual } => write!(
                f,
                "bad checksum: expected {expected:#04x}, got {actual:#04x}"
            ),
        }
    }
}

impl core::error::Error for ProtocolError {}
