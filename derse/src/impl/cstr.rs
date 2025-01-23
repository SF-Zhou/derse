use crate::*;
use std::ffi::{CStr, CString};

impl Serialize for CStr {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.to_bytes_with_nul().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for &'a CStr {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
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
    fn test_os_str() {
        let ser = CStr::from_bytes_with_nul(b"hello\0").unwrap();
        assert_eq!(ser.count_bytes(), 5);
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 7);
        let der = <&CStr>::deserialize(&bytes[..]).unwrap();
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
