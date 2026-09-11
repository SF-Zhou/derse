use crate as derse;

/// A decoding failure or an error returned by a custom serializer.
///
/// Errors are themselves serializable derived enum values. `Debug` prints the
/// same human-readable message as `Display`. An error does not imply that the
/// input or output cursor was restored to its state before the operation.
#[derive(thiserror::Error, derse::Serialize, derse::Deserialize, PartialEq, Clone)]
pub enum Error {
    /// The default placeholder value, including for a missing derived field.
    #[error("default")]
    Default,
    /// The current input view contains fewer bytes than a read requested.
    #[error("data is short for deserialize: expect {expect}, actual {actual}")]
    DataIsShort {
        /// The number of bytes requested by this read.
        expect: usize,
        /// The number of bytes remaining in the current input view.
        actual: usize,
    },
    /// A boolean tag is neither `0` nor `1`.
    #[error("invalid bool: {0}")]
    InvalidBool(u8),
    /// Invalid UTF-8 bytes, or an empty vector when a borrowed value cannot be
    /// returned because its payload was assembled into an owned buffer.
    #[error("invalid string: {0:?}")]
    InvalidString(Vec<u8>),
    /// A C-string payload lacks its final NUL or contains an interior NUL.
    #[error("invalid cstr: {0}")]
    InvalidCStr(String),
    /// All ten bytes read for a [`crate::VarInt64`] have a continuation bit.
    #[error("varint is short")]
    VarintIsShort,
    /// An enum variant name is not recognized by the requested derived type.
    #[error("invalid type: {0}")]
    InvalidType(String),
    /// A value-specific failure reported by an implementation.
    #[error("invalid value: {0}")]
    InvalidValue(String),
    /// The encoded `u32` is not a Unicode scalar value.
    #[error("invalid char: {0}")]
    InvalidChar(u32),
    /// An array element failed to decode: its zero-based index and the original
    /// error's display message. The index also counts fully decoded elements.
    #[error("invalid length: {0}, error: {1}")]
    InvalidLength(usize, String),
}

impl Default for Error {
    fn default() -> Self {
        Self::Default
    }
}

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

/// The result type used by derse's encoding, decoding, and storage interfaces.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Deserialize, DownwardBytes, Serialize};

    #[test]
    fn error_variants_preserve_values_and_messages() {
        let cases = [
            (Error::Default, "default"),
            (
                Error::DataIsShort {
                    expect: 5,
                    actual: 2,
                },
                "data is short for deserialize: expect 5, actual 2",
            ),
            (Error::InvalidBool(2), "invalid bool: 2"),
            (Error::InvalidString(vec![255]), "invalid string: [255]"),
            (
                Error::InvalidCStr("missing nul".into()),
                "invalid cstr: missing nul",
            ),
            (Error::VarintIsShort, "varint is short"),
            (Error::InvalidType("Other".into()), "invalid type: Other"),
            (Error::InvalidValue("bad".into()), "invalid value: bad"),
            (Error::InvalidChar(0x110000), "invalid char: 1114112"),
            (
                Error::InvalidLength(2, "short".into()),
                "invalid length: 2, error: short",
            ),
        ];

        for (error, message) in cases {
            assert_eq!(error.clone(), error);
            assert_eq!(error.to_string(), message);
            assert_eq!(format!("{error:?}"), message);
            let bytes = error.serialize::<DownwardBytes>().unwrap();
            assert_eq!(Error::deserialize(&bytes[..]).unwrap(), error);
        }
        assert_ne!(Error::InvalidBool(1), Error::InvalidBool(2));
        assert_ne!(Error::Default, Error::VarintIsShort);
    }

    #[test]
    fn test_error() {
        println!("{:?}", Error::InvalidBool(233));

        let ser = Error::DataIsShort {
            expect: 1,
            actual: 0,
        };
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 1 + 1 + 11 + 8 + 8);

        let der = Error::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let _ = Error::default().serialize::<DownwardBytes>().unwrap();
    }
}
