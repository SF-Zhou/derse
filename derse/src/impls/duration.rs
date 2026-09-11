//! Duration encodes u64 seconds followed by u32 subsecond nanoseconds (12 bytes).
//!
//! Excess nanoseconds are normalized into seconds. If that carry overflows,
//! decoding returns Error::InvalidValue after consuming both fields.

use crate::*;
use std::time::Duration;

impl Serialize for Duration {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        self.subsec_nanos().serialize_to(serializer)?;
        self.as_secs().serialize_to(serializer)
    }
}

impl<'a> Deserialize<'a> for Duration {
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized,
    {
        const NANOS_PER_SEC: u32 = 1_000_000_000;

        let mut secs = u64::deserialize_from(buf)?;
        let mut nanos = u32::deserialize_from(buf)?;
        if nanos >= NANOS_PER_SEC {
            secs = secs
                .checked_add(u64::from(nanos / NANOS_PER_SEC))
                .ok_or_else(|| Error::InvalidValue("duration overflow".into()))?;
            nanos %= NANOS_PER_SEC;
        }
        Ok(Self::new(secs, nanos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_wire_boundaries() {
        let cases = [
            (Duration::ZERO, [0; 12]),
            (
                Duration::MAX,
                [
                    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xc9, 0x9a, 0x3b,
                ],
            ),
        ];
        for (value, wire) in cases {
            let bytes = value.serialize::<DownwardBytes>().unwrap();
            assert_eq!(&bytes[..], &wire);
            assert_eq!(Duration::deserialize(wire.as_slice()).unwrap(), value);
        }
    }

    #[test]
    fn duration_decode_normalization_and_overflow() {
        let overflow = Err(Error::InvalidValue("duration overflow".into()));
        let cases = [
            (0, 1_000_000_000, Ok(Duration::from_secs(1))),
            (1, 1_500_000_000, Ok(Duration::new(2, 500_000_000))),
            (0, u32::MAX, Ok(Duration::new(4, 294_967_295))),
            (
                u64::MAX - 1,
                1_000_000_000,
                Ok(Duration::from_secs(u64::MAX)),
            ),
            (
                u64::MAX - 4,
                u32::MAX,
                Ok(Duration::new(u64::MAX, 294_967_295)),
            ),
            (u64::MAX, 1_000_000_000, overflow.clone()),
            (u64::MAX, u32::MAX, overflow.clone()),
            (u64::MAX - 3, u32::MAX, overflow),
        ];
        for (secs, nanos, expected) in cases {
            let mut wire = [42; 13];
            wire[..8].copy_from_slice(&secs.to_le_bytes());
            wire[8..12].copy_from_slice(&nanos.to_le_bytes());
            let mut input = wire.as_slice();
            assert_eq!(Duration::deserialize_from(&mut input), expected);
            assert_eq!(input, &[42]);

            let fragments = [&wire[..3], &wire[3..10], &wire[10..]];
            let mut input = BytesArray::new(&fragments);
            assert_eq!(Duration::deserialize_from(&mut input), expected);
            assert_eq!(input.pop(1).unwrap().as_ref(), &[42]);
        }
    }

    #[test]
    fn duration_decode_short_input_preserves_field_boundary() {
        let wire = [0; 12];
        for len in 0..12 {
            let (consumed, expect) = if len < 8 { (0, 8) } else { (8, 4) };
            let expected = Err(Error::DataIsShort {
                expect,
                actual: len - consumed,
            });
            let mut input = &wire[..len];
            assert_eq!(Duration::deserialize_from(&mut input), expected);
            assert_eq!(input, &wire[consumed..len]);

            let fragments = [&wire[..len / 2], &wire[len / 2..len]];
            let mut input = BytesArray::new(&fragments);
            assert_eq!(Duration::deserialize_from(&mut input), expected);
            assert_eq!(input.len(), len - consumed);
            assert_eq!(
                input.pop(input.len()).unwrap().as_ref(),
                &wire[consumed..len]
            );
        }
    }

    #[test]
    fn test_duration() {
        let ser = Duration::from_millis(12315);
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 12);

        let der = Duration::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }
}
