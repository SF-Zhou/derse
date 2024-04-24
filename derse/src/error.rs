#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    DataIsShort { expect: usize, actual: usize },
    InvalidBool(u8),
    InvalidString,
    VariantIsShort,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
