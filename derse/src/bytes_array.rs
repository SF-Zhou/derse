use super::{Deserializer, Error, Result};
use std::borrow::Cow;

/// Reads a slice of byte slices as one logical input without joining it up front.
///
/// [`Deserializer::advance`] creates a bounded view without copying payload
/// bytes. [`Deserializer::pop`] borrows when a nonempty read fits in the current
/// fragment; a read that crosses fragments assembles an owned `Vec<u8>`. Empty
/// reads always return a borrowed empty slice. Leading empty fragments can also
/// put a nonempty read on the owned path.
///
/// A borrowed output such as `&str` cannot retain an owned temporary produced by
/// a spanning read. Use `String` or `Cow<str>` if string payloads may be split.
/// Cloning or copying this type creates an independent cursor over the same bytes.
///
/// ```
/// use derse::{BytesArray, Deserialize};
/// use std::borrow::Cow;
///
/// let fragments: &[&[u8]] = &[b"\x05he", b"llo"];
/// let text = Cow::<str>::deserialize(BytesArray::new(fragments))?;
/// assert_eq!(text, "hello");
/// assert!(matches!(text, Cow::Owned(_)));
/// # Ok::<(), derse::Error>(())
/// ```
#[derive(Clone, Copy)]
pub struct BytesArray<'a> {
    // The first fragment starts at `pos`. The final fragment may extend beyond
    // this view; `len` is authoritative when advance() creates a shorter view.
    arr: &'a [&'a [u8]],
    pos: usize,
    len: usize,
}

impl<'a> BytesArray<'a> {
    /// Starts a cursor over all fragments, including any empty fragments.
    ///
    /// Construction sums the fragment lengths without copying their contents.
    pub fn new(arr: &'a [&[u8]]) -> Self {
        let len = arr.iter().map(|s| s.len()).sum();
        Self { arr, pos: 0, len }
    }

    /// Returns the number of unconsumed bytes in this view.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns whether this view has no unconsumed bytes.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<'a> Deserializer<'a> for BytesArray<'a> {
    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn advance(&mut self, len: usize) -> Result<Self>
    where
        Self: Sized,
    {
        if len == 0 {
            return Ok(Self {
                arr: &[],
                pos: 0,
                len: 0,
            });
        }
        if len <= self.len {
            let mut r = len;
            let mut p = self.pos;
            for (idx, s) in self.arr.iter().enumerate() {
                let c = s.len() - p;
                if r <= c {
                    let ret = Self {
                        arr: &self.arr[..idx + 1],
                        pos: self.pos,
                        len,
                    };

                    if r == c {
                        self.arr = &self.arr[idx + 1..];
                        self.pos = 0;
                        self.len -= len;
                    } else {
                        self.arr = &self.arr[idx..];
                        self.pos = p + r;
                        self.len -= len;
                    };

                    return Ok(ret);
                } else {
                    r -= c;
                    p = 0;
                }
            }
        }

        Err(Error::DataIsShort {
            expect: len,
            actual: self.len,
        })
    }

    fn pop(&mut self, len: usize) -> Result<Cow<'a, [u8]>> {
        if len == 0 {
            return Ok(Cow::Borrowed(&[]));
        }
        if len <= self.len {
            let first_slice_len = self.arr[0].len() - self.pos;
            if len <= first_slice_len {
                let s = &self.arr[0][self.pos..self.pos + len];
                if len == first_slice_len {
                    self.arr = &self.arr[1..];
                    self.pos = 0;
                } else {
                    self.pos += len;
                }
                self.len -= len;
                Ok(Cow::Borrowed(s))
            } else {
                let mut vec = Vec::from(&self.arr[0][self.pos..]);
                let mut remain = len - first_slice_len;

                self.arr = &self.arr[1..];
                self.pos = 0;
                self.len -= len;

                while remain > 0 {
                    if remain < self.arr[0].len() {
                        vec.extend_from_slice(&self.arr[0][..remain]);
                        self.pos = remain;
                        break;
                    } else {
                        remain -= self.arr[0].len();
                        vec.extend_from_slice(self.arr[0]);
                        self.arr = &self.arr[1..];
                    }
                }
                Ok(Cow::Owned(vec))
            }
        } else {
            Err(Error::DataIsShort {
                expect: len,
                actual: self.len,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_arrays_allow_zero_length_operations() {
        let inputs: [&[&[u8]]; 2] = [&[], &[&[], &[]]];
        for fragments in inputs {
            let mut arr = BytesArray::new(fragments);
            let mut front = arr.advance(0).unwrap();
            assert!(front.is_empty());
            assert!(front.advance(0).unwrap().is_empty());
            assert!(matches!(front.pop(0).unwrap(), Cow::Borrowed(b"")));
            assert!(matches!(arr.pop(0).unwrap(), Cow::Borrowed(b"")));
            assert!(arr.is_empty());
            assert!(matches!(
                arr.advance(1),
                Err(Error::DataIsShort {
                    expect: 1,
                    actual: 0
                })
            ));
            assert!(matches!(
                arr.pop(1),
                Err(Error::DataIsShort {
                    expect: 1,
                    actual: 0
                })
            ));
        }
    }

    #[test]
    fn zero_length_operations_do_not_consume_input() {
        let fragments: &[&[u8]] = &[&[], b"abc", &[], b"de", &[]];
        let mut arr = BytesArray::new(fragments);
        assert!(!Deserializer::is_empty(&arr));
        assert_eq!(arr.pop(1).unwrap().as_ref(), b"a");
        assert!(arr.advance(0).unwrap().is_empty());
        assert!(matches!(arr.pop(0).unwrap(), Cow::Borrowed(b"")));
        assert_eq!(arr.len(), 4);
        assert_eq!(arr.pop(4).unwrap().as_ref(), b"bcde");
        assert!(Deserializer::is_empty(&arr));
        assert!(arr.advance(0).unwrap().is_empty());
        assert!(matches!(arr.pop(0).unwrap(), Cow::Borrowed(b"")));
    }

    #[test]
    fn nested_views_respect_logical_boundaries() {
        let fragments: &[&[u8]] = &[&[], b"abc", &[], b"de", &[]];
        let mut arr = BytesArray::new(fragments);
        let mut front = arr.advance(4).unwrap();
        assert_eq!(front.pop(1).unwrap().as_ref(), b"a");
        let mut inner = front.advance(2).unwrap();
        assert!(matches!(
            inner.advance(3),
            Err(Error::DataIsShort {
                expect: 3,
                actual: 2
            })
        ));
        assert!(matches!(
            inner.pop(3),
            Err(Error::DataIsShort {
                expect: 3,
                actual: 2
            })
        ));
        assert_eq!(inner.pop(2).unwrap().as_ref(), b"bc");
        assert!(inner.is_empty());
        assert!(matches!(
            inner.pop(1),
            Err(Error::DataIsShort {
                expect: 1,
                actual: 0
            })
        ));
        assert!(matches!(
            front.pop(2),
            Err(Error::DataIsShort {
                expect: 2,
                actual: 1
            })
        ));
        assert_eq!(front.pop(1).unwrap().as_ref(), b"d");
        assert_eq!(arr.pop(1).unwrap().as_ref(), b"e");
    }

    #[test]
    fn test_deserializer() {
        let data = [0u8, 1, 2, 3];
        let mut slice = &data[..];

        slice.advance(2).unwrap();
        assert!(slice.advance(3).is_err());
        slice.advance(2).unwrap();
        assert!(slice.is_empty());
    }

    #[test]
    fn test_bytes_array() {
        let mut data = vec![];
        data.extend(0u8..=255);
        for _ in 0..8 {
            data.extend_from_within(..);
        }

        let mut slice = &data[..];
        let mut vec = vec![];
        while slice.len() >= 100 {
            let (front, back) = slice.split_at(100);
            vec.push(front);
            slice = back;
        }
        vec.push(slice);

        let mut arr = BytesArray::new(&vec);
        assert_eq!(arr.len(), data.len());

        let mut acc = 0;
        for i in 1..100 {
            let pop = arr.pop(i).unwrap();
            assert_eq!(pop, &data[acc..acc + i]);
            acc += i;
        }
        let pop = arr.pop(arr.len()).unwrap();
        assert_eq!(pop, &data[acc..]);

        assert!(arr.is_empty());

        let mut arr = BytesArray::new(&vec);
        assert_eq!(arr.len(), data.len());
        let mut acc = 0;
        for i in 1..100 {
            let mut front = arr.advance(i).unwrap();
            let pop = front.pop(front.len()).unwrap();
            assert_eq!(pop, &data[acc..acc + i]);
            acc += i;
        }
        let pop = arr.pop(arr.len()).unwrap();
        assert_eq!(pop, &data[acc..]);

        assert!(arr.advance(1).is_err());
    }
}
