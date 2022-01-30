//! Endianness aware reader.

use std::io::{self, Read};

/// The `FromBytes` trait allows to create a value from its representation as a
/// byte array in big and little endian.
pub trait FromBytes {
    /// Creates a value from its representation as a byte array in little
    /// endian.
    fn from_le_bytes<B: AsRef<[u8]>>(bytes: B) -> Self;

    /// Creates a value from its representation as a byte array in big
    /// endian.
    fn from_be_bytes<B: AsRef<[u8]>>(bytes: B) -> Self;
}

/// Implements the [`FromBytes`] trait for a given type.
///
/// # Panics
///
/// The default implementations panic if the provided buffer does not match the
/// size of the target type.
macro_rules! impl_from_bytes {
    ($type:ty) => {
        impl FromBytes for $type {
            fn from_le_bytes<B: AsRef<[u8]>>(bytes: B) -> Self {
                Self::from_le_bytes(
                    bytes.as_ref().try_into().expect("invalid input buffer"),
                )
            }

            fn from_be_bytes<B: AsRef<[u8]>>(bytes: B) -> Self {
                Self::from_be_bytes(
                    bytes.as_ref().try_into().expect("invalid input buffer"),
                )
            }
        }
    };
}

// Implement the FromBytes trait for all unsigned integers.
impl_from_bytes!(u8);
impl_from_bytes!(u16);
impl_from_bytes!(u32);
impl_from_bytes!(u64);
impl_from_bytes!(u128);

// Implement the FromBytes trait for all signed integers.
impl_from_bytes!(i8);
impl_from_bytes!(i16);
impl_from_bytes!(i32);
impl_from_bytes!(i64);
impl_from_bytes!(i128);

/// The `EndianRead` trait provides endianness aware read functions.
pub trait EndianRead {
    /// Reads a [`FromBytes`] value as little endian.
    fn read_le<T: FromBytes>(&mut self) -> io::Result<T>;

    /// Reads a [`FromBytes`] value as big endian.
    fn read_be<T: FromBytes>(&mut self) -> io::Result<T>;
}

impl<R: Read> EndianRead for R {
    fn read_le<T: FromBytes>(&mut self) -> io::Result<T> {
        let mut buf = vec![0u8; std::mem::size_of::<T>()];
        self.read_exact(&mut buf)?;
        Ok(T::from_le_bytes(&buf))
    }

    fn read_be<T: FromBytes>(&mut self) -> io::Result<T> {
        let mut buf = vec![0u8; std::mem::size_of::<T>()];
        self.read_exact(&mut buf)?;
        Ok(T::from_be_bytes(&buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io::Cursor;

    #[test]
    fn test_from_be_bytes() {
        let bytes = vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
        let result = <u64 as FromBytes>::from_be_bytes(bytes);
        assert_eq!(result, 0x0011223344556677);
    }

    #[test]
    fn test_from_le_bytes() {
        let bytes = vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
        let result = <u64 as FromBytes>::from_le_bytes(bytes);
        assert_eq!(result, 0x7766554433221100);
    }

    #[test]
    #[should_panic]
    fn test_from_bytes_invalid_size() {
        let bytes = vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
        <u32 as FromBytes>::from_le_bytes(bytes);
    }

    #[test]
    fn test_read_le() {
        let mut bytes =
            Cursor::new(vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]);
        let result = bytes.read_le::<u16>().unwrap();
        assert_eq!(result, 0x1100);
        let result = bytes.read_le::<u32>().unwrap();
        assert_eq!(result, 0x55443322);
    }

    #[test]
    fn test_read_be() {
        let mut bytes =
            Cursor::new(vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77]);
        let result = bytes.read_be::<u16>().unwrap();
        assert_eq!(result, 0x0011);
        let result = bytes.read_be::<u32>().unwrap();
        assert_eq!(result, 0x22334455);
    }

    #[test]
    fn test_read_oob() {
        let bytes = vec![0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
        assert!(bytes.as_slice().read_be::<u128>().is_err());
    }
}
