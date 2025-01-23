use crate::*;
use std::borrow::Cow;

macro_rules! impl_serialize_trait {
    ($($t:ty),*) => {
        $(impl Serialize for $t {
            fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
                serializer.prepend(&self.to_le_bytes())
            }
        }

        impl<'a> Deserialize<'a> for $t {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
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

impl Serialize for bool {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        serializer.prepend([*self as u8])
    }
}

impl<'a> Deserialize<'a> for bool {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
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

impl Serialize for usize {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        (*self as u64).serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for usize {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        u64::deserialize_from(buf).map(|v| v as usize)
    }
}

impl Serialize for isize {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        (*self as i64).serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for isize {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        i64::deserialize_from(buf).map(|v| v as isize)
    }
}

impl Serialize for char {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        (*self as u32).serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for char {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let c = u32::deserialize_from(buf)?;
        char::from_u32(c).ok_or(Error::InvalidChar(c))
    }
}

impl Serialize for str {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        serializer.prepend(self.as_bytes())?;
        VarInt64(self.len() as u64).serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for &'a str {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let front = buf.pop(len)?;
        match front {
            Cow::Borrowed(borrowed) => match std::str::from_utf8(borrowed) {
                Ok(str) => Ok(str),
                Err(_) => Err(Error::InvalidString(Vec::from(borrowed))),
            },
            Cow::Owned(_) => Err(Error::InvalidString(Default::default())),
        }
    }
}

impl Serialize for [u8] {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        serializer.prepend(self)?;
        VarInt64(self.len() as u64).serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for &'a [u8] {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let front = buf.pop(len)?;
        match front {
            Cow::Borrowed(borrowed) => Ok(borrowed),
            Cow::Owned(_) => Err(Error::InvalidString(Default::default())),
        }
    }
}

impl<const N: usize> Serialize for [u8; N] {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        serializer.prepend(self)
    }
}

impl<'a, const N: usize> Deserialize<'a> for [u8; N] {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let front = buf.pop(N)?;
        let mut ret = [0u8; N];
        ret.copy_from_slice(&front);
        Ok(ret)
    }
}

impl Serialize for Cow<'_, [u8]> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        <[u8]>::serialize_to(self, serializer)
    }
}

impl<'a> Deserialize<'a> for Cow<'a, [u8]> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        buf.pop(len)
    }
}

impl Serialize for Cow<'_, str> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_ref().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for Cow<'a, str> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let front = buf.pop(len)?;
        match front {
            Cow::Borrowed(borrowed) => match std::str::from_utf8(borrowed) {
                Ok(str) => Ok(Cow::Borrowed(str)),
                Err(_) => Err(Error::InvalidString(Vec::from(borrowed))),
            },
            Cow::Owned(owned) => match String::from_utf8(owned) {
                Ok(str) => Ok(Cow::Owned(str)),
                Err(e) => Err(Error::InvalidString(e.into_bytes())),
            },
        }
    }
}

impl<T: ToOwned<Owned = T> + Serialize> Serialize for Cow<'_, T> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_ref().serialize_to(serializer)
    }
}

impl<'a, T: ToOwned<Owned = T> + Deserialize<'a>> Deserialize<'a> for Cow<'a, T> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Cow::Owned(T::deserialize_from(buf)?))
    }
}

impl Serialize for String {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_str().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for String {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Cow::<str>::deserialize_from(buf)?.into_owned())
    }
}

impl<Item: Serialize> Serialize for Option<Item> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        if let Some(item) = self {
            item.serialize_to(serializer)?;
            true.serialize_to(serializer)
        } else {
            false.serialize_to(serializer)
        }
    }
}

impl<'a, Item: Deserialize<'a>> Deserialize<'a> for Option<Item> {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
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

impl<T: Serialize + ?Sized> Serialize for &T {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        T::serialize_to(self, serializer)
    }
}

impl Serialize for () {
    fn serialize_to<S: Serializer>(&self, _: &mut S) -> Result<()> {
        Ok(())
    }
}

impl<'a> Deserialize<'a> for () {
    fn deserialize_from<D: Deserializer<'a>>(_: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(())
    }
}

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
            "data is short for deserialize: expect 1, actual 0".to_owned()
        );

        {
            let ser = 233isize;
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = isize::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            isize::deserialize([0xff].as_slice()).unwrap_err();
        }

        {
            let ser = '😊';
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = char::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            char::deserialize([0xff].as_slice()).unwrap_err();
            char::deserialize([0, 0, 0x11, 0].as_slice()).unwrap_err();
        }

        {
            let ser = "hello world!";
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der: String = String::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, &der);

            let der = Cow::<str>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, &der);

            let der = Cow::<[u8]>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser.len(), der.len());
            let bytes: DownwardBytes = der.serialize().unwrap();

            let der = Cow::<String>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der.as_ref());
            let bytes: DownwardBytes = der.serialize().unwrap();

            let der: &str = Deserialize::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            let bytes2: DownwardBytes = der.serialize().unwrap();
            assert_eq!(bytes, bytes2);

            assert!(Cow::<str>::deserialize([2, 0xC0, 0xAF].as_slice()).is_err());
            assert!(Cow::<str>::deserialize([128].as_slice()).is_err());

            let result: Result<&str> = Deserialize::deserialize([2, 0xC0, 0xAF].as_slice());
            assert!(result.is_err());
        }

        {
            let ser = "hello world!".to_string();
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der: String = String::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            assert!(String::deserialize(&bytes[..1]).is_err());
            assert!(String::deserialize(&bytes[..5]).is_err());
        }

        {
            assert!(u32::deserialize([0, 1, 2].as_ref()).is_err());
        }

        {
            let ser = (String::from("hello"), 64u32);
            let bytes: DownwardBytes = ser.serialize().unwrap();
            assert_eq!(bytes.len(), 1 + 5 + 4);
            let der: (String, u32) = Deserialize::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            let der: (String, u16, u16) = Deserialize::deserialize(&bytes[..]).unwrap();
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

            let result: Result<&str> = Deserialize::deserialize(BytesArray::new(&c));
            assert!(result.is_err());

            assert!(String::deserialize(BytesArray::new(&c[1..])).is_err());

            let a = [0x2, 0xC0];
            let b = [0xAF];
            assert!(Cow::<str>::deserialize(BytesArray::new(&[&a[..], &b[..]])).is_err());
        }

        {
            let ser = ();
            let bytes = ser.serialize::<DownwardBytes>().unwrap();
            assert!(bytes.is_empty());
            <()>::deserialize(&bytes[..]).unwrap();
        }

        {
            let ser = 233u32;
            let bytes = ser.serialize::<DownwardBytes>().unwrap();
            let array = <[u8; 4]>::deserialize(&bytes[..]).unwrap();
            let der = u32::from_le_bytes(array);
            assert_eq!(ser, der);
        }
    }
}
