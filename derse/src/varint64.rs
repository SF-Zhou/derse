use super::{Deserialize, Error, Result, Serialize};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct VarInt64(pub u64);

const B: u8 = 7;
const M: u8 = (1 << B) - 1;

impl Serialize for VarInt64 {
    fn serialize_to<S: crate::Serializer>(&self, serializer: &mut S) -> Result<()> {
        let mut v = self.0;
        serializer.prepend([(v as u8) & M])?;
        v >>= B;

        while v != 0 {
            serializer.prepend([v as u8 | (1 << B)])?;
            v >>= B;
        }

        Ok(())
    }
}

impl<'a> Deserialize<'a> for VarInt64 {
    fn deserialize_from<D: crate::Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        let mut v = 0u64;
        for _ in 0..10 {
            let front = buf.pop(1)?;
            let c = front[0];
            v = (v << B) | (c & M) as u64;
            if c & (1 << B) == 0 {
                return Ok(Self(v));
            }
        }
        Err(Error::VarintIsShort)
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_varint64() {
        for v in [u64::MIN, 1, 10, 127, 128, 255, 256, u64::MAX] {
            let ser = VarInt64(v);
            let bytes = ser.serialize::<DownwardBytes>().unwrap();
            let der = VarInt64::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);
        }

        assert!(VarInt64::deserialize(&[][..]).is_err());
        assert!(VarInt64::deserialize(&[128u8; 11][..]).is_err());
    }
}
