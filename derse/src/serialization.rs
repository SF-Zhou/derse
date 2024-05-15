use std::borrow::Cow;

use super::{Deserializer, Error, Result, Serializer, VarInt64};
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;

pub trait Serialization<'a> {
    fn serialize<S: Serializer + Default>(&self) -> Result<S> {
        let mut serializer = S::default();
        self.serialize_to(&mut serializer)?;
        Ok(serializer)
    }

    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()>;

    fn deserialize<S: Deserializer<'a>>(mut der: S) -> Result<Self>
    where
        Self: Sized,
    {
        Self::deserialize_from(&mut der)
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
    where
        Self: Sized;
}

macro_rules! impl_serialize_trait {
    ($($t:ty),*) => {
        $(impl<'a> Serialization<'a> for $t {
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                serializer.prepend(&self.to_le_bytes())
            }

            fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
            where
                Self: Sized,
            {
                let front = buf.pop(std::mem::size_of::<Self>())?;
                Ok(Self::from_le_bytes(front.as_ref().try_into().unwrap()))
            }
        })*
    };
}

impl_serialize_trait! {i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64}

impl<'a> Serialization<'a> for bool {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        serializer.prepend([*self as u8])
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
    where
        Self: Sized,
    {
        let front = buf.pop(1)?;
        match front[0] {
            0 => Ok(false),
            1 => Ok(true),
            v => Err(Error::InvalidBool(v)),
        }
    }
}

impl<'a> Serialization<'a> for &'a str {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        serializer.prepend(self.as_bytes())?;
        VarInt64(self.len() as u64).serialize_to(serializer)
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let front = buf.pop(len)?;
        match front {
            Cow::Borrowed(borrowed) => match std::str::from_utf8(borrowed) {
                Ok(str) => Ok(str),
                Err(_) => Err(Error::InvalidString),
            },
            Cow::Owned(_) => Err(Error::InvalidString),
        }
    }
}

impl<'a> Serialization<'a> for Cow<'a, str> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_ref().serialize_to(serializer)
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let front = buf.pop(len)?;
        match front {
            Cow::Borrowed(borrowed) => match std::str::from_utf8(borrowed) {
                Ok(str) => Ok(Cow::Borrowed(str)),
                Err(_) => Err(Error::InvalidString),
            },
            Cow::Owned(owned) => match std::str::from_utf8(&owned) {
                Ok(_) => Ok(Cow::Owned(unsafe { String::from_utf8_unchecked(owned) })),
                Err(_) => Err(Error::InvalidString),
            },
        }
    }
}

impl<'a> Serialization<'a> for String {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_str().serialize_to(serializer)
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Cow::<str>::deserialize_from(buf)?.into_owned())
    }
}

impl<'a, Item: Serialization<'a>> Serialization<'a> for Vec<Item> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        for item in self.iter().rev() {
            item.serialize_to(serializer)?;
        }
        VarInt64(self.len() as u64).serialize_to(serializer)
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
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
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        for item in self.iter() {
            item.serialize_to(serializer)?;
        }
        VarInt64(self.len() as u64).serialize_to(serializer)
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
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

impl<'a, K: Eq + Hash + Serialization<'a>, V: Serialization<'a>> Serialization<'a>
    for HashMap<K, V>
{
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        for item in self.iter() {
            item.serialize_to(serializer)?;
        }
        VarInt64(self.len() as u64).serialize_to(serializer)
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let mut out = HashMap::with_capacity(len);
        for _ in 0..len {
            let key = K::deserialize_from(buf)?;
            let value = V::deserialize_from(buf)?;
            out.insert(key, value);
        }
        Ok(out)
    }
}

impl<'a, Item: Serialization<'a>> Serialization<'a> for Option<Item> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        if let Some(item) = self {
            item.serialize_to(serializer)?;
            true.serialize_to(serializer)
        } else {
            false.serialize_to(serializer)
        }
    }

    fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
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

impl<'a, T: Serialization<'a>> Serialization<'a> for &T {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        T::serialize_to(self, serializer)
    }

    fn deserialize_from<S: Deserializer<'a>>(_buf: &mut S) -> Result<Self>
    where
        Self: Sized,
    {
        Err(Error::InvalidType)
    }
}

macro_rules! tuple_serialization {
    (($($name:ident),+), ($($idx:tt),+)) => {
        impl<'a, $($name),+> Serialization<'a> for ($($name,)+)
        where
            $($name: Serialization<'a>),+
        {
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                $((self.$idx.serialize_to(serializer))?;)+
                Ok(())
            }

            fn deserialize_from<S: Deserializer<'a>>(buf: &mut S) -> Result<Self>
            where
                Self: Sized,
            {
                Ok(($($name::deserialize_from(buf)?,)+))
            }
        }
    };
}

tuple_serialization!((A), (0));
tuple_serialization!((A, B), (1, 0));
tuple_serialization!((A, B, C), (2, 1, 0));
tuple_serialization!((A, B, C, D), (3, 2, 1, 0));

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    #[test]
    fn test_serialization() {
        for ser in [u64::MIN, u64::MAX] {
            let bytes = ser.serialize::<DownwardBytes>().unwrap();
            assert_eq!(bytes.len(), 8);

            let der = u64::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);
        }

        assert!(bool::deserialize(true.serialize::<DownwardBytes>().unwrap().as_ref()).unwrap());
        assert!(!bool::deserialize(false.serialize::<DownwardBytes>().unwrap().as_ref()).unwrap());
        assert!(!bool::deserialize(&[0][..]).unwrap());
        assert!(bool::deserialize(&[1][..]).unwrap());
        assert!(bool::deserialize(&[2][..]).is_err());
        assert!(bool::deserialize(&[][..]).is_err());
        assert_eq!(
            bool::deserialize(&[][..]).unwrap_err().to_string(),
            "DataIsShort { expect: 1, actual: 0 }".to_owned()
        );

        {
            let ser = "hello world!";
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der: String = String::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, &der);

            let der = Cow::<str>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, &der);

            let der: &str = Serialization::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            let bytes2: DownwardBytes = der.serialize().unwrap();
            assert_eq!(bytes, bytes2);

            assert!(Cow::<str>::deserialize([2, 0xC0, 0xAF].as_slice()).is_err());
            assert!(Cow::<str>::deserialize([128].as_slice()).is_err());

            let result: Result<&str> = Serialization::deserialize([2, 0xC0, 0xAF].as_slice());
            assert!(result.is_err());
        }

        {
            let ser = "hello world!".to_string();
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der: String = String::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            assert!(matches!(
                <&String>::deserialize(&bytes[..]).unwrap_err(),
                Error::InvalidType
            ));

            assert!(String::deserialize(&bytes[..1]).is_err());
            assert!(String::deserialize(&bytes[..5]).is_err());
        }

        {
            let ser = vec!["hello", "world", "!"];
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = Vec::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            assert!(Vec::<u8>::deserialize([128].as_ref()).is_err());
            assert!(Vec::<u8>::deserialize([1].as_ref()).is_err());
            assert!(Vec::<u8>::deserialize([0].as_ref()).unwrap().is_empty());
        }

        {
            let ser: HashSet<String> = "hello world !".split(' ').map(|s| s.to_owned()).collect();
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = HashSet::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            assert!(HashSet::<u8>::deserialize([128].as_ref()).is_err());
            assert!(HashSet::<u8>::deserialize([1].as_ref()).is_err());
            assert!(HashSet::<u8>::deserialize([0].as_ref()).unwrap().is_empty());
        }

        {
            let ser = Some("hello".to_string());
            let bytes: DownwardBytes = ser.serialize().unwrap();
            assert_eq!(bytes.len(), 1 + 1 + 5);
            let der = Option::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);
            let der = Vec::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser.as_ref(), der.first());

            let ser = None;
            let bytes: DownwardBytes = ser.serialize().unwrap();
            assert_eq!(bytes.len(), 1);
            let der = Option::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);
            let der = Vec::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser.as_ref(), der.first());

            assert!(Option::<String>::deserialize([128].as_ref()).is_err());
            assert!(Option::<String>::deserialize([1].as_ref()).is_err());
            assert!(Option::<String>::deserialize([0].as_ref())
                .unwrap()
                .is_none());
        }

        {
            assert!(u32::deserialize([0, 1, 2].as_ref()).is_err());
        }

        {
            let ser: HashMap<String, u32> = (0..10).map(|i| (i.to_string(), i)).collect();
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = HashMap::<String, u32>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            let mut der = Vec::<(String, u32)>::deserialize(&bytes[..]).unwrap();
            assert_eq!(der.len(), 10);
            der.sort();
            assert_eq!(der[0].0, "0");
            assert_eq!(der[9].0, "9");
        }

        {
            let ser = (String::from("hello"), 64u32);
            let bytes: DownwardBytes = ser.serialize().unwrap();
            assert_eq!(bytes.len(), 1 + 5 + 4);
            let der: (String, u32) = Serialization::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            let der: (String, u16, u16) = Serialization::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser.0, der.0);
            assert_eq!(ser.1, der.1 as _);
            assert_eq!(0, der.2);
        }

        {
            let msg = "0".repeat(47) + "A";
            let a = msg[..25].as_bytes();
            let b = msg[24..].as_bytes();
            let c = [a, b];

            let der = String::deserialize(BytesArray::new(&c)).unwrap();
            assert_eq!(msg, der);

            let der = Cow::<str>::deserialize(BytesArray::new(&c)).unwrap();
            assert_eq!(msg, der);

            let result: Result<&str> = Serialization::deserialize(BytesArray::new(&c));
            assert!(result.is_err());

            assert!(String::deserialize(BytesArray::new(&c[1..])).is_err());

            let a = [0x2, 0xC0];
            let b = [0xAF];
            assert!(Cow::<str>::deserialize(BytesArray::new(&[&a[..], &b[..]])).is_err());
        }
    }
}
