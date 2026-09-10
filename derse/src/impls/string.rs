//! UTF-8 strings and raw byte slices use a VarInt64 byte length before the payload.
//!
//! Borrowed results require pop() to return borrowed bytes; an owned temporary
//! cannot be lent for the input lifetime. That case currently uses InvalidString
//! with an empty payload, even for raw bytes. Owned String decoding goes through
//! `Cow<str>` so that fragmented payloads can be assembled and validated.

use crate::*;
use std::borrow::Cow;

impl Serialize for str {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        serializer.prepend(self.as_bytes())?;
        VarInt64(self.len() as u64).serialize_to(serializer)
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a str {
    fn deserialize_from<D: Deserializer<'de>>(buf: &mut D) -> Result<Self>
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

impl<'de: 'a, 'a> Deserialize<'de> for &'a [u8] {
    fn deserialize_from<D: Deserializer<'de>>(buf: &mut D) -> Result<Self>
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
    fn borrowed_values_report_truncated_length_and_payload() {
        for bytes in [&[128][..], b"\x03a".as_slice()] {
            assert!(<&str>::deserialize(bytes).is_err());
            assert!(<&[u8]>::deserialize(bytes).is_err());
            let fragments = [bytes];
            assert!(<&str>::deserialize(BytesArray::new(&fragments)).is_err());
            assert!(<&[u8]>::deserialize(BytesArray::new(&fragments)).is_err());
        }
        let invalid: &[&[u8]] = &[b"\x02\xc0\xaf"];
        assert_eq!(
            <&str>::deserialize(BytesArray::new(invalid)),
            Err(Error::InvalidString(vec![0xc0, 0xaf]))
        );
    }

    #[test]
    fn strings_and_byte_slices_stop_after_a_failed_payload_write() {
        let mut serializer = crate::serializer::tests::FailingSerializer::default();
        assert_eq!(
            "abc".serialize_to(&mut serializer),
            Err(Error::InvalidValue("write failed".into()))
        );
        assert_eq!(serializer.writes, 1);

        let mut serializer = crate::serializer::tests::FailingSerializer::default();
        assert_eq!(
            b"abc".as_slice().serialize_to(&mut serializer),
            Err(Error::InvalidValue("write failed".into()))
        );
        assert_eq!(serializer.writes, 1);
    }

    #[test]
    fn borrowed_values_can_have_shorter_lifetimes_than_the_input() {
        fn deserialize_shorter<'de: 'a, 'a>(bytes: &'de [u8]) -> (&'a str, &'a [u8]) {
            (
                <&'a str as Deserialize<'de>>::deserialize(bytes).unwrap(),
                <&'a [u8] as Deserialize<'de>>::deserialize(bytes).unwrap(),
            )
        }

        let bytes = b"\x03abc";
        let (text, data) = deserialize_shorter(bytes);
        assert_eq!(text, "abc");
        assert_eq!(data, b"abc");
        assert_eq!(text.as_ptr(), bytes[1..].as_ptr());
        assert_eq!(data.as_ptr(), bytes[1..].as_ptr());
    }

    #[test]
    fn empty_values_from_fragmented_input() {
        let fragments: &[&[u8]] = &[&[0]];
        assert_eq!(<&str>::deserialize(BytesArray::new(fragments)).unwrap(), "");
        assert_eq!(
            <&[u8]>::deserialize(BytesArray::new(fragments)).unwrap(),
            b""
        );
        assert!(matches!(
            Cow::<str>::deserialize(BytesArray::new(fragments)).unwrap(),
            Cow::Borrowed("")
        ));
        assert!(matches!(
            Cow::<[u8]>::deserialize(BytesArray::new(fragments)).unwrap(),
            Cow::Borrowed(b"")
        ));
    }

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
