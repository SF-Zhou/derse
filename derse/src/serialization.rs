use std::borrow::Cow;

use super::{Error, Result, Serializer, VarInt64};

pub trait Serialization<'a> {
    fn serialize<T: Serializer + Default>(&self) -> T {
        let mut serializer = T::default();
        self.serialize_to(&mut serializer);
        serializer
    }

    fn serialize_to<T: Serializer>(&self, serializer: &mut T);

    fn deserialize(mut buf: &'a [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        Self::deserialize_from(&mut buf)
    }

    fn deserialize_from(buf: &mut &'a [u8]) -> Result<Self>
    where
        Self: Sized;
}

macro_rules! impl_serialize_trait {
    ($($t:ty),*) => {
        $(impl Serialization<'_> for $t {
            fn serialize_to<T: Serializer>(&self, serializer: &mut T) {
                serializer.prepend(&self.to_le_bytes());
            }

            fn deserialize_from(buf: &mut &[u8]) -> Result<Self> {
                if buf.len() < std::mem::size_of::<Self>() {
                    Err(Error::DataIsShort {
                        expect: std::mem::size_of::<Self>(),
                        actual: buf.len(),
                    })
                } else {
                    let (front, back) = buf.split_at(std::mem::size_of::<Self>());
                    *buf = back;
                    Ok(Self::from_le_bytes(front.try_into().unwrap()))
                }
            }
        })*
    };
}

impl_serialize_trait! {i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64}

impl Serialization<'_> for bool {
    fn serialize_to<T: Serializer>(&self, serializer: &mut T) {
        serializer.prepend([*self as u8])
    }

    fn deserialize_from(buf: &mut &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        if buf.is_empty() {
            Err(Error::DataIsShort {
                expect: 1,
                actual: 0,
            })
        } else {
            match buf[0] {
                0 => {
                    *buf = &buf[1..];
                    Ok(false)
                }
                1 => {
                    *buf = &buf[1..];
                    Ok(true)
                }
                v => Err(Error::InvalidBool(v)),
            }
        }
    }
}

impl<'a> Serialization<'a> for &'a str {
    fn serialize_to<T: Serializer>(&self, serializer: &mut T) {
        serializer.prepend(self.as_bytes());
        VarInt64(self.len() as u64).serialize_to(serializer);
    }

    fn deserialize_from(buf: &mut &'a [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        if buf.len() >= len {
            let (front, back) = buf.split_at(len);
            *buf = back;
            match std::str::from_utf8(front) {
                Ok(str) => Ok(str),
                Err(_) => Err(Error::InvalidString),
            }
        } else {
            Err(Error::DataIsShort {
                expect: len,
                actual: buf.len(),
            })
        }
    }
}

impl<'a> Serialization<'a> for Cow<'a, str> {
    fn serialize_to<T: Serializer>(&self, serializer: &mut T) {
        self.as_ref().serialize_to(serializer);
    }

    fn deserialize_from(buf: &mut &'a [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        type T<'a> = &'a str;
        T::deserialize_from(buf).map(Cow::Borrowed)
    }
}

impl Serialization<'_> for String {
    fn serialize_to<T: Serializer>(&self, serializer: &mut T) {
        self.as_str().serialize_to(serializer)
    }

    fn deserialize_from(buf: &mut &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        type T<'a> = &'a str;
        T::deserialize_from(buf).map(|s| s.to_string())
    }
}

impl<'a, Item: Serialization<'a>> Serialization<'a> for Vec<Item> {
    fn serialize_to<T: Serializer>(&self, serializer: &mut T) {
        for item in self.iter().rev() {
            item.serialize_to(serializer);
        }
        VarInt64(self.len() as u64).serialize_to(serializer);
    }

    fn deserialize_from(buf: &mut &'a [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.push(Item::deserialize_from(buf)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_serialization() {
        use super::super::*;
        use super::*;

        for ser in [u64::MIN, u64::MAX] {
            let bytes = ser.serialize::<DownwardBytes>();
            assert_eq!(bytes.len(), 8);

            let der = u64::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);
        }

        assert!(bool::deserialize(&true.serialize::<DownwardBytes>()).unwrap());
        assert!(!bool::deserialize(&false.serialize::<DownwardBytes>()).unwrap());
        assert!(!bool::deserialize(&[0]).unwrap());
        assert!(bool::deserialize(&[1]).unwrap());
        assert!(bool::deserialize(&[2]).is_err());
        assert!(bool::deserialize(&[]).is_err());

        {
            let ser = "hello world!";
            let bytes: DownwardBytes = ser.serialize();
            let der: String = String::deserialize(&bytes).unwrap();
            assert_eq!(ser, &der);

            let der = Cow::<str>::deserialize(&bytes).unwrap();
            assert_eq!(ser, &der);

            let bytes2: DownwardBytes = der.serialize();
            assert_eq!(bytes, bytes2);

            let bytes = [2, 0xC0, 0xAF];
            assert!(Cow::<str>::deserialize(&bytes).is_err());

            let bytes = [128];
            assert!(Cow::<str>::deserialize(&bytes).is_err());
        }

        {
            let ser = "hello world!".to_string();
            let bytes: DownwardBytes = ser.serialize();
            let der: String = String::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);

            assert!(String::deserialize(&bytes[..1]).is_err());
            assert!(String::deserialize(&bytes[..5]).is_err());
        }

        {
            let ser = vec!["hello", "world", "!"];
            let bytes: DownwardBytes = ser.serialize();
            let der = Vec::<String>::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);

            let bytes = vec![128];
            assert!(Vec::<u8>::deserialize(&bytes).is_err());

            let bytes = vec![1];
            assert!(Vec::<u8>::deserialize(&bytes).is_err());
        }

        {
            assert!(u32::deserialize(&[0, 1, 2]).is_err());
        }
    }
}
