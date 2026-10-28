#![cfg(feature = "thin-vec")]

use derse::{
    BytesArray, Deserialize, Deserializer, DownwardBytes, Error, Serialize, Serializer, VarInt64,
};
use thin_vec::ThinVec;

#[test]
fn encoding_matches_vec_and_decoders_are_interchangeable() {
    for len in [0, 1, 3, 127, 128] {
        let values: Vec<u16> = (0..len).map(|index| index as u16 * 17 + 3).collect();
        let thin: ThinVec<_> = values.iter().copied().collect();
        // UFCS checks the actual trait implementation, without slice autoderef.
        let bytes = Serialize::serialize::<DownwardBytes>(&thin).unwrap();
        let vec_bytes = values.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.as_slice(), vec_bytes.as_slice());
        assert_eq!(Serialize::serialize::<usize>(&thin).unwrap(), bytes.len());
        assert_eq!(Vec::<u16>::deserialize(bytes.as_slice()).unwrap(), values);
        assert_eq!(
            ThinVec::<u16>::deserialize(vec_bytes.as_slice()).unwrap(),
            thin
        );

        let fragments: Vec<_> = bytes.chunks(3).collect();
        assert_eq!(
            ThinVec::<u16>::deserialize(BytesArray::new(&fragments)).unwrap(),
            thin
        );

        let mut with_tail = bytes.as_slice().to_vec();
        with_tail.push(99);
        let mut input = with_tail.as_slice();
        assert_eq!(ThinVec::<u16>::deserialize_from(&mut input).unwrap(), thin);
        assert_eq!(input, &[99]);
    }

    let value: ThinVec<u16> = [0x1234, 0x5678].into_iter().collect();
    assert_eq!(
        Serialize::serialize::<DownwardBytes>(&value)
            .unwrap()
            .as_slice(),
        &[2, 0x34, 0x12, 0x78, 0x56]
    );
}

#[test]
fn borrowed_elements_and_fragmented_owned_elements_are_supported() {
    fn decode<'de: 'a, 'a>(input: &'de [u8]) -> ThinVec<&'a str> {
        <ThinVec<&'a str> as Deserialize<'de>>::deserialize(input).unwrap()
    }

    let input = Vec::from(b"\x02\x03one\x03two");
    let borrowed = decode(&input);
    assert_eq!(borrowed.as_slice(), &["one", "two"]);
    assert_eq!(borrowed[0].as_ptr(), input[2..].as_ptr());
    assert_eq!(borrowed[1].as_ptr(), input[6..].as_ptr());
    assert_eq!(
        Serialize::serialize::<DownwardBytes>(&borrowed)
            .unwrap()
            .as_slice(),
        input
    );

    let fragments: &[&[u8]] = &[b"\x02\x03o", b"ne\x03t", b"wo"];
    let owned = ThinVec::<String>::deserialize(BytesArray::new(fragments)).unwrap();
    assert_eq!(owned.as_slice(), &["one", "two"]);
    assert_eq!(
        Serialize::serialize::<DownwardBytes>(&owned)
            .unwrap()
            .as_slice(),
        input
    );
}

#[test]
fn derived_generic_fields_do_not_require_clone_or_default_elements() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Element(#[derse(required)] u16);

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Message<T> {
        #[derse(required)]
        values: ThinVec<T>,
    }

    let value = Message {
        values: [Element(3), Element(7)].into_iter().collect(),
    };
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(
        Message::<Element>::deserialize(bytes.as_slice()).unwrap(),
        value
    );
}

#[test]
fn malformed_input_propagates_errors_and_preserves_cursor_progress() {
    assert_eq!(
        ThinVec::<u8>::deserialize(&[128][..]),
        Err(Error::DataIsShort {
            expect: 1,
            actual: 0
        })
    );

    let mut input = &[2, 1, 0, 9][..];
    assert_eq!(
        ThinVec::<u16>::deserialize_from(&mut input),
        Err(Error::DataIsShort {
            expect: 2,
            actual: 1
        })
    );
    assert_eq!(input, &[9]);

    let mut input = &[3, 1, 2, 99][..];
    assert_eq!(
        ThinVec::<bool>::deserialize_from(&mut input),
        Err(Error::InvalidBool(2))
    );
    assert_eq!(input, &[99]);
}

#[test]
fn serialization_stops_after_the_first_failed_element_write() {
    #[derive(Default)]
    struct RejectWrites(Vec<Vec<u8>>);

    impl Serializer for RejectWrites {
        fn prepend(&mut self, data: impl AsRef<[u8]>) -> derse::Result<()> {
            self.0.push(data.as_ref().to_vec());
            Err(Error::InvalidValue("write failed".into()))
        }

        fn len(&self) -> usize {
            0
        }
    }

    let mut serializer = RejectWrites::default();
    let value: ThinVec<u8> = [1, 2].into_iter().collect();
    assert_eq!(
        Serialize::serialize_to(&value, &mut serializer),
        Err(Error::InvalidValue("write failed".into()))
    );
    assert_eq!(serializer.0, [vec![2]]);
}

#[test]
fn large_counts_do_not_preallocate_and_zero_sized_elements_work() {
    for count in [1u64 << 32, u64::MAX] {
        let bytes = VarInt64(count).serialize::<DownwardBytes>().unwrap();
        let mut input = bytes.as_slice();
        assert_eq!(
            ThinVec::<u8>::deserialize_from(&mut input),
            Err(Error::DataIsShort {
                expect: 1,
                actual: 0
            })
        );
        assert!(input.is_empty());
    }

    let mut input = &[3, 99][..];
    let units = ThinVec::<()>::deserialize_from(&mut input).unwrap();
    assert_eq!(units.len(), 3);
    assert_eq!(
        Serialize::serialize::<DownwardBytes>(&units)
            .unwrap()
            .as_slice(),
        &[3]
    );
    assert_eq!(input, &[99]);
}

#[test]
fn initialized_elements_are_dropped_after_errors_and_panics() {
    use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

    struct Element;
    static DROPPED: AtomicUsize = AtomicUsize::new(0);

    impl Drop for Element {
        fn drop(&mut self) {
            DROPPED.fetch_add(1, SeqCst);
        }
    }

    impl<'a> Deserialize<'a> for Element {
        fn deserialize_from<D: Deserializer<'a>>(input: &mut D) -> derse::Result<Self> {
            match u8::deserialize_from(input)? {
                255 => Err(Error::InvalidBool(255)),
                254 => panic!("element decoder panicked"),
                _ => Ok(Self),
            }
        }
    }

    let mut input = &[4, 1, 2, 255, 99][..];
    assert!(matches!(
        ThinVec::<Element>::deserialize_from(&mut input),
        Err(Error::InvalidBool(255))
    ));
    assert_eq!(DROPPED.load(SeqCst), 2);
    assert_eq!(input, &[99]);

    DROPPED.store(0, SeqCst);
    let mut input = &[4, 1, 2, 254, 99][..];
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ThinVec::<Element>::deserialize_from(&mut input)
    }))
    .is_err());
    assert_eq!(DROPPED.load(SeqCst), 2);
    assert_eq!(input, &[99]);
}
