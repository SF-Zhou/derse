use super::{Deserialize, Error, Result, Serialize};

/// An unsigned integer encoded as one to ten base-128 digits, most significant first.
///
/// Each byte contributes seven value bits. Its high bit is set if another byte
/// follows, so `127` encodes as `[0x7f]` and `128` as `[0x81, 0x00]`. Zero occupies
/// one byte. This is not the least-significant-digit-first encoding used by LEB128.
/// Derse uses this type for byte lengths and collection element counts.
///
/// Serialization emits the shortest representation. Deserialization accepts
/// nonminimal encodings and does not reject overflow in a ten-byte sequence;
/// high bits shifted out of the `u64` accumulator are discarded. A continuation
/// bit on the tenth byte produces [`Error::VarintIsShort`]. Missing input bytes
/// instead propagate the input's read error.
///
/// ```
/// use derse::{Deserialize, DownwardBytes, Serialize, VarInt64};
///
/// let bytes: DownwardBytes = VarInt64(128).serialize()?;
/// assert_eq!(bytes.as_slice(), &[0x81, 0x00]);
/// assert_eq!(VarInt64::deserialize(bytes.as_slice())?, VarInt64(128));
/// # Ok::<(), derse::Error>(())
/// ```
#[derive(Debug, Default, PartialEq, Eq)]
pub struct VarInt64(
    /// The unsigned value to encode or the decoded value.
    pub u64,
);

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
