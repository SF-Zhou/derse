#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    DataIsShort { expect: usize, actual: usize },
    InvalidBool(u8),
    InvalidString,
    VariantIsShort,
}

pub type Result<T> = std::result::Result<T, Error>;
