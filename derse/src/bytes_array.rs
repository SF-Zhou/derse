use super::{Deserializer, Error, Result};
use std::borrow::Cow;

#[derive(Clone, Copy)]
pub struct BytesArray<'a> {
    arr: &'a [&'a [u8]],
    pos: usize,
    len: usize,
}

impl<'a> BytesArray<'a> {
    pub fn new(arr: &'a [&[u8]]) -> Self {
        let len = arr.iter().map(|s| s.len()).sum();
        Self { arr, pos: 0, len }
    }

    pub fn len(&self) -> usize {
        self.len
    }

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
            actual: 0,
        })
    }

    fn pop(&mut self, len: usize) -> Result<Cow<'a, [u8]>> {
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
