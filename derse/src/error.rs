#[derive(thiserror::Error)]
pub enum Error {
    #[error("data is short for deserialize: expect {expect}, actual {actual}")]
    DataIsShort { expect: usize, actual: usize },
    #[error("invalid bool {0}")]
    InvalidBool(u8),
    #[error("invalid string {0:?}")]
    InvalidString(Vec<u8>),
    #[error("varint is short")]
    VarintIsShort,
    #[error("invalid type")]
    InvalidType,
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

    #[test]
    fn test_error() {
        println!("{:?}", Error::InvalidBool(233));
    }
}
