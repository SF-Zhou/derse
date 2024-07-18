use std::borrow::Cow;

use super::{Error, Result};

/// A trait for deserializing data from a byte slice.
pub trait Deserializer<'a> {
    /// Checks if the deserializer is empty.
    fn is_empty(&self) -> bool;

    /// Advances the deserializer by the specified length.
    ///
    /// # Errors
    ///
    /// Returns an error if the length to advance exceeds the available data.
    fn advance(&mut self, len: usize) -> Result<Self>
    where
        Self: Sized;

    /// Pops the specified length of data from the deserializer.
    ///
    /// # Errors
    ///
    /// Returns an error if the length to pop exceeds the available data.
    fn pop(&mut self, len: usize) -> Result<Cow<'a, [u8]>>;
}

/// Implements the `Deserializer` trait for a byte slice.
impl<'a> Deserializer<'a> for &'a [u8] {
    /// Checks if the byte slice is empty.
    fn is_empty(&self) -> bool {
        <[u8]>::is_empty(self)
    }

    /// Advances the byte slice by the specified length.
    ///
    /// # Errors
    ///
    /// Returns an error if the length to advance exceeds the available data.
    fn advance(&mut self, len: usize) -> Result<Self>
    where
        Self: Sized,
    {
        if len <= self.len() {
            let (front, back) = self.split_at(len);
            *self = back;
            Ok(front)
        } else {
            Err(Error::DataIsShort {
                expect: len,
                actual: self.len(),
            })
        }
    }

    /// Pops the specified length of data from the byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the length to pop exceeds the available data.
    fn pop(&mut self, len: usize) -> Result<Cow<'a, [u8]>> {
        if len <= self.len() {
            let (front, back) = self.split_at(len);
            *self = back;
            Ok(Cow::Borrowed(front))
        } else {
            Err(Error::DataIsShort {
                expect: len,
                actual: self.len(),
            })
        }
    }
}
