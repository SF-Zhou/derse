use crate::*;
use std::borrow::Cow;

impl Serialize for Cow<'_, [u8]> {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        <[u8]>::serialize_to(self, serializer)
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for Cow<'a, [u8]> {
    fn deserialize_from<D: Deserializer<'de>>(buf: &mut D) -> Result<Self>
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

impl<'de: 'a, 'a> Deserialize<'de> for Cow<'a, str> {
    fn deserialize_from<D: Deserializer<'de>>(buf: &mut D) -> Result<Self>
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

impl<'de, 'a, T: ToOwned<Owned = T> + Deserialize<'de>> Deserialize<'de> for Cow<'a, T> {
    fn deserialize_from<D: Deserializer<'de>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        Ok(Cow::Owned(T::deserialize_from(buf)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cow_decoding_preserves_ownership_and_reports_truncation() {
        let contiguous: &[&[u8]] = &[b"\x03abc"];
        assert!(matches!(
            Cow::<str>::deserialize(BytesArray::new(contiguous)).unwrap(),
            Cow::Borrowed("abc")
        ));
        assert!(matches!(
            Cow::<[u8]>::deserialize(BytesArray::new(contiguous)).unwrap(),
            Cow::Borrowed(b"abc")
        ));
        let fragments: &[&[u8]] = &[b"\x03a", b"bc"];
        let text = Cow::<str>::deserialize(BytesArray::new(fragments)).unwrap();
        let data = Cow::<[u8]>::deserialize(BytesArray::new(fragments)).unwrap();
        assert!(matches!(text, Cow::Owned(_)));
        assert!(matches!(data, Cow::Owned(_)));
        assert_eq!(text, "abc");
        assert_eq!(data.as_ref(), b"abc");

        for text in [Cow::Borrowed("abc"), Cow::Owned("abc".into())] {
            let bytes = text.serialize::<DownwardBytes>().unwrap();
            assert_eq!(&bytes[..], b"\x03abc");
        }
        for data in [
            Cow::Borrowed(b"abc".as_slice()),
            Cow::Owned(b"abc".to_vec()),
        ] {
            let bytes = data.serialize::<DownwardBytes>().unwrap();
            assert_eq!(&bytes[..], b"\x03abc");
        }

        for bytes in [&[128][..], b"\x03a".as_slice()] {
            assert!(Cow::<[u8]>::deserialize(bytes).is_err());
            assert!(Cow::<str>::deserialize(bytes).is_err());
            assert!(Cow::<String>::deserialize(bytes).is_err());
            let fragments = [bytes];
            assert!(Cow::<[u8]>::deserialize(BytesArray::new(&fragments)).is_err());
            assert!(Cow::<str>::deserialize(BytesArray::new(&fragments)).is_err());
        }
        for fragments in [
            &[b"\x02\xc0\xaf".as_slice()][..],
            &[b"\x02\xc0".as_slice(), b"\xaf".as_slice()][..],
        ] {
            assert_eq!(
                Cow::<str>::deserialize(BytesArray::new(fragments)),
                Err(Error::InvalidString(vec![0xc0, 0xaf]))
            );
        }
    }

    #[test]
    fn borrowed_cows_can_have_shorter_lifetimes_than_the_input() {
        fn deserialize_shorter<'de: 'a, 'a>(bytes: &'de [u8]) -> (Cow<'a, str>, Cow<'a, [u8]>) {
            (
                <Cow<'a, str> as Deserialize<'de>>::deserialize(bytes).unwrap(),
                <Cow<'a, [u8]> as Deserialize<'de>>::deserialize(bytes).unwrap(),
            )
        }

        let bytes = b"\x03abc";
        let (text, data) = deserialize_shorter(bytes);
        assert!(matches!(text, Cow::Borrowed("abc")));
        assert!(matches!(data, Cow::Borrowed(b"abc")));
        assert_eq!(text.as_ptr(), bytes[1..].as_ptr());
        assert_eq!(data.as_ptr(), bytes[1..].as_ptr());
    }

    #[test]
    // Exercise the generic Cow<T> implementation with an owned string.
    #[allow(clippy::owned_cow)]
    fn owned_cow_lifetime_is_independent_of_the_input() {
        fn deserialize_owned<'de>(bytes: &'de [u8]) -> Cow<'static, String> {
            <Cow<'static, String> as Deserialize<'de>>::deserialize(bytes).unwrap()
        }

        let text = {
            let bytes = Vec::from(b"\x03abc");
            deserialize_owned(&bytes)
        };
        assert!(matches!(text, Cow::Owned(_)));
        assert_eq!(text.as_str(), "abc");
    }
}
