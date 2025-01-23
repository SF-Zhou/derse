use crate::{Deserialize, Serialize, VarInt64};

impl<A: tinyvec::Array> Serialize for tinyvec::TinyVec<A>
where
    A::Item: Serialize,
{
    fn serialize_to<S: crate::Serializer>(&self, serializer: &mut S) -> crate::Result<()> {
        for item in self.iter().rev() {
            item.serialize_to(serializer)?;
        }
        VarInt64(self.len() as u64).serialize_to(serializer)
    }
}

impl<'a, A: tinyvec::Array> Deserialize<'a> for tinyvec::TinyVec<A>
where
    A::Item: Deserialize<'a>,
{
    fn deserialize_from<D: crate::Deserializer<'a>>(buf: &mut D) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let len = VarInt64::deserialize_from(buf)?.0 as usize;
        let mut out = Self::with_capacity(len);
        for _ in 0..len {
            out.push(Deserialize::deserialize_from(buf)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_compact_str() {
        type Bytes = tinyvec::TinyVec<[u8; 14]>;
        let ser = Bytes::from(b"hello".as_slice());
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        let der = Bytes::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
