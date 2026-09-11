use std::borrow::Cow;

use super::{Error, Result};

/// A forward-only input cursor that can lend bytes for the lifetime `'a`.
///
/// A successful [`pop`](Self::pop) or [`advance`](Self::advance) removes exactly
/// the requested number of bytes from the front. Zero-length operations must
/// succeed even at the end of input. Returned data must respect any logical
/// boundary of the cursor, including a view created by `advance`.
///
/// `&[u8]` always lends contiguous bytes. [`BytesArray`](crate::BytesArray) reads
/// across multiple slices and may allocate when `pop` spans a fragment boundary.
/// Implementations should report [`Error::DataIsShort`] when there are too few
/// bytes. The built-in cursors leave their position unchanged on that error.
pub trait Deserializer<'a> {
    /// Returns whether no bytes remain in this cursor's logical view.
    fn is_empty(&self) -> bool;

    /// Removes the next `len` bytes and returns a cursor limited to those bytes.
    ///
    /// Reading the returned cursor does not move this cursor further. Derived
    /// decoders use this operation to prevent a field from reading beyond its
    /// containing value, while the outer cursor already points to the next value.
    fn advance(&mut self, len: usize) -> Result<Self>
    where
        Self: Sized;

    /// Removes and returns exactly the next `len` bytes.
    ///
    /// Return `Cow::Borrowed` when the bytes can be lent for `'a`, or `Cow::Owned`
    /// when they must be assembled into a contiguous allocation. Decoders for
    /// borrowed output types can reject an owned result.
    fn pop(&mut self, len: usize) -> Result<Cow<'a, [u8]>>;
}

/// Reads borrowed bytes by replacing the slice with its unconsumed suffix.
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
