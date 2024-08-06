use crate::{Deserialize, Serialize};

impl Serialize for compact_str::CompactString {
    fn serialize_to<S: crate::Serializer>(&self, serializer: &mut S) -> crate::Result<()> {
        self.as_str().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for compact_str::CompactString {
    fn deserialize_from<D: crate::Deserializer<'a>>(buf: &mut D) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let str: &str = Deserialize::deserialize_from(buf)?;
        Ok(compact_str::CompactString::new(str))
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_compact_str() {
        let ser = compact_str::CompactString::const_new("hello");
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        let der = compact_str::CompactString::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
