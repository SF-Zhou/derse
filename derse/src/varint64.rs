use super::{Error, Result, Serialization};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct VarInt64(pub u64);

const B: u8 = 7;
const M: u8 = (1 << B) - 1;

impl Serialization<'_> for VarInt64 {
    fn serialize_to<T: crate::Serializer>(&self, serializer: &mut T) {
        let mut v = self.0;
        serializer.prepend([(v as u8) & M]);
        v >>= B;

        while v != 0 {
            serializer.prepend([v as u8 | (1 << B)]);
            v >>= B;
        }
    }

    fn deserialize_from(buf: &mut &[u8]) -> Result<Self>
    where
        Self: Sized,
    {
        let mut v = 0u64;
        for _ in 0..10 {
            if let Some(c) = buf.first() {
                *buf = &buf[1..];
                v = (v << B) | (c & M) as u64;
                if c & (1 << B) == 0 {
                    return Ok(Self(v));
                }
            } else {
                return Err(Error::VariantIsShort);
            }
        }
        Err(Error::VariantIsShort)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_varint64() {
        use crate::*;

        for v in [u64::MIN, 1, 10, 127, 128, 255, 256, u64::MAX] {
            let ser = VarInt64(v);
            let bytes = ser.serialize::<DownwardBytes>();
            let der = VarInt64::deserialize(&bytes).unwrap();
            assert_eq!(ser, der);
        }

        assert!(VarInt64::deserialize(&[]).is_err());
        assert!(VarInt64::deserialize(&[128u8; 11]).is_err());
    }
}
