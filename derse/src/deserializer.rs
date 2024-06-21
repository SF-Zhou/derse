use std::borrow::Cow;

use super::{Error, Result};

pub trait Deserializer<'a> {
    fn is_empty(&self) -> bool;

    fn advance(&mut self, len: usize) -> Result<Self>
    where
        Self: Sized;

    fn pop(&mut self, len: usize) -> Result<Cow<'a, [u8]>>;
}

impl<'a> Deserializer<'a> for &'a [u8] {
    fn is_empty(&self) -> bool {
        <[u8]>::is_empty(self)
    }

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
