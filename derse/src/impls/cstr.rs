use crate::*;
use std::ffi::{CStr, CString};

impl Serialize for CStr {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.to_bytes_with_nul().serialize_to(serializer)
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a CStr {
    fn deserialize_from<D: Deserializer<'de>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let bytes: &[u8] = Deserialize::deserialize_from(buf)?;
        CStr::from_bytes_with_nul(bytes).map_err(|e| Error::InvalidCStr(e.to_string()))
    }
}

impl Serialize for CString {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.as_ref().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for CString {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let bytes: Vec<u8> = Deserialize::deserialize_from(buf)?;
        CString::from_vec_with_nul(bytes).map_err(|e| Error::InvalidCStr(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_strings_reject_invalid_terminators_and_truncation() {
        for bytes in [b"\x03abc".as_slice(), b"\x03a\0\0".as_slice()] {
            assert!(matches!(
                <&CStr>::deserialize(bytes),
                Err(Error::InvalidCStr(_))
            ));
            assert!(matches!(
                CString::deserialize(bytes),
                Err(Error::InvalidCStr(_))
            ));
        }
        for bytes in [&[128][..], b"\x03a".as_slice()] {
            assert!(<&CStr>::deserialize(bytes).is_err());
            assert!(CString::deserialize(bytes).is_err());
        }

        let fragments: &[&[u8]] = &[b"\x04ab", b"c\0"];
        let decoded = CString::deserialize(BytesArray::new(fragments)).unwrap();
        assert_eq!(decoded.to_bytes_with_nul(), b"abc\0");
        assert!(<&CStr>::deserialize(BytesArray::new(fragments)).is_err());
    }

    #[test]
    fn test_os_str() {
        fn deserialize_shorter<'de: 'a, 'a>(bytes: &'de [u8]) -> &'a CStr {
            <&'a CStr as Deserialize<'de>>::deserialize(bytes).unwrap()
        }

        let ser = CStr::from_bytes_with_nul(b"hello\0").unwrap();
        assert_eq!(ser.count_bytes(), 5);
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 7);
        let der = deserialize_shorter(&bytes[..]);
        assert_eq!(ser, der);

        let der = CString::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der.as_c_str());

        let ser = ser.to_owned();
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 7);
        let der = CString::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
