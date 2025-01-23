use crate::*;

macro_rules! primitive_impl {
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

primitive_impl! {i8, i16, i32, i64, i128, u8, u16, u32, u64, u128, f32, f64}

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

impl<T: Serialize + ?Sized> Serialize for &T {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        T::serialize_to(self, serializer)
    }
}

impl<T: Serialize + ?Sized> Serialize for &mut T {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_types() {
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
            let mut ser = '😊';
            let r = &mut ser;
            let bytes: DownwardBytes = r.serialize().unwrap();
            let der = char::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            char::deserialize([0xff].as_slice()).unwrap_err();
            char::deserialize([0, 0, 0x11, 0].as_slice()).unwrap_err();
        }

        {
            let ser = ();
            let bytes = ser.serialize::<DownwardBytes>().unwrap();
            assert!(bytes.is_empty());
            <()>::deserialize(&bytes[..]).unwrap();
        }

        {
            assert!(u32::deserialize([0, 1, 2].as_ref()).is_err());
        }
    }
}
