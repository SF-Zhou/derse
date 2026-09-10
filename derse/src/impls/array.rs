use crate::*;

const MAX_ARRAY_LENGTH: usize = 32;

impl<T: Serialize, const N: usize> Serialize for [T; N] {
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()> {
        // Keep the array impl for every N so byte arrays cannot silently fall
        // back to the slice impl. Reject unsupported lengths during codegen.
        const {
            assert!(
                N <= MAX_ARRAY_LENGTH,
                "array serialization supports at most 32 elements"
            );
        }
        for item in self.iter().rev() {
            item.serialize_to(serializer)?;
        }
        Ok(())
    }
}

macro_rules! array_impls {
    // Each length expands to a literal array; Rust drops initialized elements
    // if a later element returns an error or panics.
    ([$($index:expr,)*] $len:expr $(, $rest:expr)*) => {
        impl<'a, T: Deserialize<'a>> Deserialize<'a> for [T; $len] {
            fn deserialize_from<D: Deserializer<'a>>(_buf: &mut D) -> Result<Self> {
                Ok([$(
                    match T::deserialize_from(_buf) {
                        Ok(value) => value,
                        Err(error) => return Err(Error::InvalidLength($index, error.to_string())),
                    }
                ),*])
            }
        }

        array_impls!([$($index,)* $len,] $($rest),*);
    };
    ([$($index:expr,)*]) => {};
}

array_impls!([]
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    MAX_ARRAY_LENGTH
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_arrays_do_not_consume_input() {
        let value: [u8; 0] = [];
        assert!(value.serialize::<DownwardBytes>().unwrap().is_empty());

        let mut input = &[42][..];
        assert_eq!(<[u8; 0]>::deserialize_from(&mut input).unwrap(), []);
        assert_eq!(input, &[42]);
        assert_eq!(<[u8; 0]>::deserialize(BytesArray::new(&[])).unwrap(), []);
    }

    #[test]
    fn arrays_decode_at_the_length_limit() {
        let value: [u16; 32] = std::array::from_fn(|index| index as u16 + 1);
        let bytes = value.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 64);
        assert_eq!(&bytes[..4], &[1, 0, 2, 0]);
        assert_eq!(&bytes[62..], &[32, 0]);
        assert_eq!(<[u16; 32]>::deserialize(&bytes[..]).unwrap(), value);

        let fragments: Vec<_> = bytes.chunks(3).collect();
        assert_eq!(
            <[u16; 32]>::deserialize(BytesArray::new(&fragments)).unwrap(),
            value
        );

        let borrowed = ["first", "second", "third"];
        let bytes = borrowed.serialize::<DownwardBytes>().unwrap();
        assert_eq!(<[&str; 3]>::deserialize(&bytes[..]).unwrap(), borrowed);
    }

    #[test]
    fn initialized_elements_are_dropped_on_error_and_panic() {
        use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

        // No Copy, Clone or Default implementation is required for array elements.
        struct Element;
        static DROPPED: AtomicUsize = AtomicUsize::new(0);

        impl Drop for Element {
            fn drop(&mut self) {
                DROPPED.fetch_add(1, SeqCst);
            }
        }

        impl<'a> Deserialize<'a> for Element {
            fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self> {
                match u8::deserialize_from(buf)? {
                    255 => Err(Error::InvalidBool(255)),
                    254 => panic!("element decoder panicked"),
                    _ => Ok(Self),
                }
            }
        }

        for index in [0, 1, 31] {
            DROPPED.store(0, SeqCst);
            let mut bytes = [1; 35];
            bytes[index] = 255;
            let mut input = bytes.as_slice();
            let result = <[Element; 32]>::deserialize_from(&mut input);
            assert!(
                matches!(result, Err(Error::InvalidLength(actual, ref error))
                if actual == index && *error == Error::InvalidBool(255).to_string())
            );
            assert_eq!(input, &bytes[index + 1..]);
            assert_eq!(DROPPED.load(SeqCst), index);
        }

        DROPPED.store(0, SeqCst);
        let mut input = &[1, 2, 254, 4][..];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            <[Element; 4]>::deserialize_from(&mut input)
        }));
        assert!(result.is_err());
        assert_eq!(input, &[4]);
        assert_eq!(DROPPED.load(SeqCst), 2);

        DROPPED.store(0, SeqCst);
        let elements = <[Element; 3]>::deserialize(&[1, 2, 3][..]).unwrap();
        assert_eq!(DROPPED.load(SeqCst), 0);
        drop(elements);
        assert_eq!(DROPPED.load(SeqCst), 3);
    }

    #[test]
    fn test_array() {
        {
            let ser: [i32; 3] = [1, 2, 3];
            let bytes: DownwardBytes = ser.serialize().unwrap();
            let der = <[i32; 3]>::deserialize(&bytes[..]).unwrap();
            assert_eq!(ser, der);

            <[bool; 3]>::deserialize([].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([1].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([2].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([1, 1].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([1, 2].as_slice()).unwrap_err();
            <[bool; 3]>::deserialize([1, 1, 1].as_slice()).unwrap();
            <[bool; 3]>::deserialize([1, 1, 2].as_slice()).unwrap_err();
        }

        {
            let ser = 233u32;
            let bytes = ser.serialize::<DownwardBytes>().unwrap();
            let array = <[u8; 4]>::deserialize(&bytes[..]).unwrap();
            let der = u32::from_le_bytes(array);
            assert_eq!(ser, der);
        }
    }
}
