use crate::*;
use std::borrow::Cow;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string() {
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
            assert!(<&[u8]>::deserialize(BytesArray::new(&[&a[..], &b[..]])).is_err());
            assert!(Cow::<str>::deserialize(BytesArray::new(&[&a[..], &b[..]])).is_err());
        }
    }
}
