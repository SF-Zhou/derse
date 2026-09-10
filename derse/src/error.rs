use crate as derse;

#[derive(thiserror::Error, derse::Serialize, derse::Deserialize, PartialEq, Clone)]
pub enum Error {
    #[error("default")]
    Default,
    #[error("data is short for deserialize: expect {expect}, actual {actual}")]
    DataIsShort { expect: usize, actual: usize },
    #[error("invalid bool: {0}")]
    InvalidBool(u8),
    #[error("invalid string: {0:?}")]
    InvalidString(Vec<u8>),
    #[error("invalid cstr: {0}")]
    InvalidCStr(String),
    #[error("varint is short")]
    VarintIsShort,
    #[error("invalid type: {0}")]
    InvalidType(String),
    #[error("invalid value: {0}")]
    InvalidValue(String),
    #[error("invalid char: {0}")]
    InvalidChar(u32),
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
