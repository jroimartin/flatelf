//! Custom error type.

use std::fmt;
use std::io;
use std::num;

/// A flatelf specific error.
pub enum Error {
    /// Invalid ELF magic.
    InvalidElfMagic,

    /// Invalid ELF version.
    InvalidElfVersion,

    /// Invalid ELF CPU word size.
    InvalidSize,

    /// Invalid ELF endianness.
    InvalidEndian,

    /// Invalid file or memory offset.
    InvalidOffset,

    /// No LOAD segments.
    NoLoadSegments,

    /// Invalid type conversion.
    InvalidTypeConversion,

    /// IO operation error.
    Io(io::Error),
}

impl From<num::TryFromIntError> for Error {
    fn from(_err: num::TryFromIntError) -> Error {
        Error::InvalidTypeConversion
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidElfMagic => write!(f, "invalid ELF magic"),
            Error::InvalidElfVersion => write!(f, "invalid ELF version"),
            Error::InvalidSize => write!(f, "invalid CPU word size"),
            Error::InvalidEndian => write!(f, "invalid endianness"),
            Error::InvalidOffset => write!(f, "invalid file or memory offset"),
            Error::NoLoadSegments => write!(f, "no LOAD segments"),
            Error::InvalidTypeConversion => {
                write!(f, "invalid type conversion")
            }
            Error::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}
