use std::borrow::Cow;

use super::{Error, Result, Serializer, VarInt64};
use std::collections::HashSet;
use std::hash::Hash;

pub trait Serialization<'a> {
    fn serialize<S: Serializer + Default>(&self) -> S {
        let mut serializer = S::default();
        self.serialize_to(&mut serializer);
        serializer
    }

    fn serialize_to<S: Serializer>(&self, serializer: &mut S);

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
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
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
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
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
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
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

impl<'a> Serialization<'a> for &'a [u8] {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
        serializer.prepend(self);
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
            Ok(front)
        } else {
            Err(Error::DataIsShort {
                expect: len,
                actual: buf.len(),
            })
        }
    }
}

impl<'a, T: ToOwned + ?Sized> Serialization<'a> for Cow<'a, T>
where
    for<'b> &'b T: Serialization<'b>,
{
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
        self.as_ref().serialize_to(serializer);
    }

    fn deserialize_from(buf: &mut &'a [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        Serialization::deserialize_from(buf).map(Cow::Borrowed)
    }
}

impl Serialization<'_> for String {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
        self.as_str().serialize_to(serializer)
    }

    fn deserialize_from(buf: &mut &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        Serialization::deserialize_from(buf).map(|s: &str| s.to_string())
    }
}

impl<'a, Item: Serialization<'a>> Serialization<'a> for Vec<Item> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
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

impl<'a, Item: Eq + Hash + Serialization<'a>> Serialization<'a> for HashSet<Item> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
        for item in self.iter() {
            item.serialize_to(serializer);
        }
        VarInt64(self.len() as u64).serialize_to(serializer);
    }

    fn deserialize_from(buf: &mut &'a [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let mut out = HashSet::with_capacity(len);
        for _ in 0..len {
            out.insert(Item::deserialize_from(buf)?);
        }
        Ok(out)
    }
}

impl<'a, Item: Serialization<'a>> Serialization<'a> for Option<Item> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) {
        if let Some(item) = self {
            item.serialize_to(serializer);
            true.serialize_to(serializer);
        } else {
            false.serialize_to(serializer);
        }
    }

    fn deserialize_from(buf: &mut &'a [u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let has = bool::deserialize_from(buf)?;
        if has {
            Ok(Some(Item::deserialize_from(buf)?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn test_serialization() {
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
        assert_eq!(
            bool::deserialize(&[]).unwrap_err().to_string(),
            "DataIsShort { expect: 1, actual: 0 }".to_owned()
        );

        {
            let ser = "hello world!";
            let bytes: DownwardBytes = ser.serialize();
            let der: String = String::deserialize(&bytes).unwrap();
            assert_eq!(ser, &der);

            let der = Cow::<str>::deserialize(&bytes).unwrap();
            assert_eq!(ser, &der);

            let bytes2: DownwardBytes = der.serialize();
            assert_eq!(bytes, bytes2);

            assert!(Cow::<str>::deserialize(&[2, 0xC0, 0xAF]).is_err());
            assert!(Cow::<str>::deserialize(&[128]).is_err());
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

            assert!(Vec::<u8>::deserialize(&[128]).is_err());
            assert!(Vec::<u8>::deserialize(&[1]).is_err());
            assert!(Vec::<u8>::deserialize(&[0]).unwrap().is_empty());
        }

        {
            let ser: HashSet<String> = "hello world !".split(' ').map(|s| s.to_owned()).collect();
            let bytes: DownwardBytes = ser.serialize();
            let der = HashSet::<String>::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);

            assert!(HashSet::<u8>::deserialize(&[128]).is_err());
            assert!(HashSet::<u8>::deserialize(&[1]).is_err());
            assert!(HashSet::<u8>::deserialize(&[0]).unwrap().is_empty());
        }

        {
            let ser = Some("hello".to_string());
            let bytes: DownwardBytes = ser.serialize();
            assert_eq!(bytes.len(), 1 + 1 + 5);
            let der = Option::<String>::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);
            let der = Vec::<String>::deserialize(&bytes).unwrap();
            assert_eq!(ser.as_ref(), der.first());

            let ser = None;
            let bytes: DownwardBytes = ser.serialize();
            assert_eq!(bytes.len(), 1);
            let der = Option::<String>::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);
            let der = Vec::<String>::deserialize(&bytes).unwrap();
            assert_eq!(ser.as_ref(), der.first());

            assert!(Option::<String>::deserialize(&[128]).is_err());
            assert!(Option::<String>::deserialize(&[1]).is_err());
            assert!(Option::<String>::deserialize(&[0]).unwrap().is_none());
        }

        {
            let ser = "hello".as_bytes();
            let bytes: DownwardBytes = ser.serialize();
            assert_eq!(bytes.len(), 1 + 5);
            let der: &[u8] = Serialization::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);

            let result: Result<&[u8]> = Serialization::deserialize(&[128u8]);
            assert!(result.is_err());
            let result: Result<&[u8]> = Serialization::deserialize(&[1u8]);
            assert!(result.is_err());

            let ser = Cow::Borrowed("hello".as_bytes());
            let bytes: DownwardBytes = ser.serialize();
            assert_eq!(bytes.len(), 1 + 5);
            let der: Cow<[u8]> = Serialization::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);
        }

        {
            assert!(u32::deserialize(&[0, 1, 2]).is_err());
        }
    }
}
