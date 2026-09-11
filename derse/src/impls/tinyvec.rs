//! TinyVec uses Vec's count-prefixed element encoding, independent of its inline
//! capacity. Decoding reserves for the declared count before reading elements.
//! An unrepresentable allocation size returns Error::InvalidValue after the
//! count is consumed. Valid allocation sizes retain the allocator's usual
//! out-of-memory behavior; this is not a configurable resource limit.

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
        // Match Vec's allocation-size limit without first constructing an inline
        // array. Zero-sized elements require no allocation regardless of count.
        std::alloc::Layout::array::<A::Item>(len)
            .map_err(|_| crate::Error::InvalidValue("tinyvec capacity overflow".into()))?;
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
    fn test_tinyvec() {
        type Bytes = tinyvec::TinyVec<[u8; 14]>;
        let ser = Bytes::from(b"hello".as_slice());
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        let der = Bytes::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser, der);
    }

    #[test]
    fn oversized_counts_return_an_error_before_reading_elements() {
        fn reject<A: tinyvec::Array>(count: u64)
        where
            A::Item: for<'a> Deserialize<'a>,
        {
            let mut bytes = DownwardBytes::new();
            bytes.prepend([99]);
            VarInt64(count).serialize_to(&mut bytes).unwrap();
            let mut input = bytes.as_slice();
            assert!(matches!(
                tinyvec::TinyVec::<A>::deserialize_from(&mut input),
                Err(Error::InvalidValue(message)) if message == "tinyvec capacity overflow"
            ));
            assert_eq!(input, &[99]);
        }

        reject::<[u8; 4]>(u64::MAX);
        reject::<[u8; 4]>(isize::MAX as u64 + 1);
        reject::<[u64; 4]>(isize::MAX as u64 / 8 + 1);
        reject::<[u64; 4]>(usize::MAX as u64 / 8 + 1);
    }

    #[test]
    fn decoding_preserves_inline_and_heap_initialization() {
        use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

        struct Item(u8);
        static DEFAULTS: AtomicUsize = AtomicUsize::new(0);

        impl Default for Item {
            fn default() -> Self {
                DEFAULTS.fetch_add(1, SeqCst);
                Self(0)
            }
        }

        impl<'a> Deserialize<'a> for Item {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
                u8::deserialize_from(buf).map(Self)
            }
        }

        let heap = tinyvec::TinyVec::<[Item; 4]>::deserialize(&[5, 1, 2, 3, 4, 5][..]).unwrap();
        assert!(heap.is_heap());
        assert_eq!(DEFAULTS.load(SeqCst), 0);
        assert_eq!(
            heap.iter().map(|item| item.0).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );

        let inline = tinyvec::TinyVec::<[Item; 4]>::deserialize(&[1, 9][..]).unwrap();
        assert!(inline.is_inline());
        assert_eq!(DEFAULTS.load(SeqCst), 4);
        assert_eq!(inline[0].0, 9);
    }

    #[test]
    fn zero_sized_elements_do_not_impose_an_allocation_limit() {
        #[derive(Default)]
        struct Empty;

        impl<'a> Deserialize<'a> for Empty {
            fn deserialize_from<D: Deserializer<'a>>(_: &mut D) -> Result<Self> {
                Err(Error::InvalidValue("element decode failed".into()))
            }
        }

        let empty = tinyvec::TinyVec::<[Empty; 4]>::deserialize(&[0][..]).unwrap();
        assert!(empty.is_inline());
        assert!(empty.is_empty());

        // Stop at the first element to check the maximum count without a huge
        // loop. A zero-sized allocation must reach the decoder, not overflow.
        let bytes = VarInt64(u64::MAX).serialize::<DownwardBytes>().unwrap();
        assert!(matches!(
            tinyvec::TinyVec::<[Empty; 4]>::deserialize(bytes.as_slice()),
            Err(Error::InvalidValue(message)) if message == "element decode failed"
        ));

        let mut input = &[5, 99][..];
        let units = tinyvec::TinyVec::<[(); 4]>::deserialize_from(&mut input).unwrap();
        assert!(units.is_heap());
        assert_eq!(units.len(), 5);
        assert_eq!(input, &[99]);
    }
}
