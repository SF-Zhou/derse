use crate::*;
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    os::unix::ffi::{OsStrExt, OsStringExt},
};

impl Serialize for OsStr {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_bytes().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for &'a OsStr {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let front = buf.pop(len)?;
        match front {
            Cow::Borrowed(borrowed) => Ok(OsStr::from_bytes(borrowed)),
            Cow::Owned(_) => Err(Error::InvalidString(Default::default())),
        }
    }
}

impl Serialize for OsString {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_os_str().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for OsString {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let front = buf.pop(len)?;
        Ok(OsString::from_vec(front.into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_str() {
        let path = std::env::current_dir().unwrap();
        let ser = path.as_os_str();

        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        let der = <&OsStr>::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let ser = ser.to_owned();
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        let der = OsString::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);

        let msg = "0".repeat(47) + "A";
        let a = msg[..25].as_bytes();
        let b = msg[24..].as_bytes();
        let c = [a, b];
        <&OsStr>::deserialize(BytesArray::new(&c)).unwrap_err();
    }
}
