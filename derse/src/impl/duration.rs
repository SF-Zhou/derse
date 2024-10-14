use crate::*;
use std::time::Duration;

impl Serialize for Duration {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.subsec_nanos().serialize_to(serializer)?;
        self.as_secs().serialize_to(serializer)
    }
}

impl Deserialize for Duration {
    type Output<'a> = Self;

    fn deserialize_from<'a, D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let secs = u64::deserialize_from(buf)?;
        let nanos = u32::deserialize_from(buf)?;
        Ok(Self::new(secs, nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_duration() {
        let ser = Duration::from_millis(12315);
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 12);

        let der = Duration::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
