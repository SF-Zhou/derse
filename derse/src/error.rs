use crate as derse;

#[derive(thiserror::Error, derse::serialize, derse::deserialize, PartialEq)]
pub enum Error {
    #[error("data is short for deserialize: expect {expect}, actual {actual}")]
    DataIsShort { expect: usize, actual: usize },
    #[error("invalid bool: {0}")]
    InvalidBool(u8),
    #[error("invalid string: {0:?}")]
    InvalidString(Vec<u8>),
    #[error("varint is short")]
    VarintIsShort,
    #[error("invalid type: {0}")]
    InvalidType(String),
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
    }
}
