//! OS strings encode their raw Unix bytes with a VarInt64 byte-length prefix.
//!
//! No UTF-8 conversion or validation occurs. These unconditional Unix extension
//! imports currently make the crate Unix-only. Borrowed OsStr decoding needs a
//! borrowed payload; OsString accepts an assembled owned payload as well.

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

impl<'de: 'a, 'a> Deserialize<'de> for &'a OsStr {
    fn deserialize_from<D: Deserializer<'de>>(buf: &mut D) -> Result<Self>
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
    fn os_strings_preserve_non_utf8_bytes_and_report_truncation() {
        let encoded = b"\x03a\xffb";
        let borrowed = <&OsStr>::deserialize(&encoded[..]).unwrap();
        assert_eq!(borrowed.as_bytes(), &encoded[1..]);
        assert_eq!(borrowed.as_bytes().as_ptr(), encoded[1..].as_ptr());
        let contiguous: &[&[u8]] = &[encoded];
        assert_eq!(
            <&OsStr>::deserialize(BytesArray::new(contiguous)).unwrap(),
            borrowed
        );

        let fragments: &[&[u8]] = &[&encoded[..2], &encoded[2..]];
        let owned = OsString::deserialize(BytesArray::new(fragments)).unwrap();
        assert_eq!(owned.as_bytes(), borrowed.as_bytes());
        assert_eq!(
            owned.serialize::<DownwardBytes>().unwrap().as_ref(),
            encoded
        );
        assert!(matches!(
            <&OsStr>::deserialize(BytesArray::new(fragments)),
            Err(Error::InvalidString(_))
        ));

        for bytes in [&[128][..], b"\x03a".as_slice()] {
            assert!(<&OsStr>::deserialize(bytes).is_err());
            assert!(OsString::deserialize(bytes).is_err());
            let fragments = [bytes];
            assert!(<&OsStr>::deserialize(BytesArray::new(&fragments)).is_err());
            assert!(OsString::deserialize(BytesArray::new(&fragments)).is_err());
        }
    }

    #[test]
    fn test_os_str() {
        fn deserialize_shorter<'de: 'a, 'a>(bytes: &'de [u8]) -> &'a OsStr {
            <&'a OsStr as Deserialize<'de>>::deserialize(bytes).unwrap()
        }

        let path = std::env::current_dir().unwrap();
        let ser = path.as_os_str();

        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        let der = deserialize_shorter(&bytes[..]);
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
