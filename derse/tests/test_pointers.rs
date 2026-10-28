use std::{
    ffi::{CStr, CString, OsStr, OsString},
    fmt::Debug,
    marker::PhantomData,
    os::unix::ffi::OsStringExt,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use derse::{BytesArray, Deserialize, DownwardBytes, Error, Serialize, Serializer};

// A generic bound ensures these tests exercise the pointer's trait implementation,
// rather than succeeding through method-call autoderef to its contents.
fn round_trip<T>(value: T, expected: &[u8])
where
    T: Serialize + for<'a> Deserialize<'a> + Debug + PartialEq,
{
    let bytes = T::serialize::<DownwardBytes>(&value).unwrap();
    assert_eq!(bytes.as_ref(), expected);
    assert_eq!(T::serialize::<usize>(&value).unwrap(), expected.len());
    assert_eq!(T::deserialize(bytes.as_slice()).unwrap(), value);

    let fragments: Vec<_> = expected.chunks(1).collect();
    assert_eq!(T::deserialize(BytesArray::new(&fragments)).unwrap(), value);

    let mut with_tail = expected.to_vec();
    with_tail.push(99);
    let mut input = with_tail.as_slice();
    assert_eq!(T::deserialize_from(&mut input).unwrap(), value);
    assert_eq!(input, &[99]);
}

// This payload is neither Clone nor Default, and its marker makes it neither
// Send nor Sync. Pointer support must not impose any of those bounds.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Borrowed<'a> {
    #[derse(required)]
    text: &'a str,
    marker: PhantomData<Rc<()>>,
}

#[derive(Default)]
struct RejectWrites(usize);

impl Serializer for RejectWrites {
    fn prepend(&mut self, _: impl AsRef<[u8]>) -> derse::Result<()> {
        self.0 += 1;
        Err(Error::InvalidValue("write failed".into()))
    }

    fn len(&self) -> usize {
        0
    }
}

macro_rules! pointer_tests {
    ($module:ident, $pointer:ident) => {
        mod $module {
            use super::*;

            #[test]
            fn pointers_are_transparent_in_generic_code_and_derived_fields() {
                round_trip($pointer::new(0x1234u16), &[0x34, 0x12]);
                round_trip($pointer::new(()), &[]);
                round_trip($pointer::new([1u8, 2, 3]), &[1, 2, 3]);
                round_trip($pointer::new(Some($pointer::new(7u8))), &[1, 7]);

                #[derive(Debug, PartialEq, Serialize, Deserialize)]
                struct Message<T> {
                    #[derse(required)]
                    value: $pointer<T>,
                }

                round_trip(
                    Message {
                        value: $pointer::new(0x1234u16),
                    },
                    &[2, 0x34, 0x12],
                );
            }

            #[test]
            fn owned_unsized_targets_accept_fragmented_input() {
                for text in ["", "héllo"] {
                    let mut expected = vec![text.len() as u8];
                    expected.extend_from_slice(text.as_bytes());
                    round_trip($pointer::<str>::from(text), &expected);
                    round_trip($pointer::<[u8]>::from(text.as_bytes()), &expected);
                }

                round_trip(
                    $pointer::<CStr>::from(CString::new("abc").unwrap()),
                    b"\x04abc\0",
                );
                round_trip(
                    $pointer::<OsStr>::from(OsString::from_vec(b"a\xffb".to_vec())),
                    b"\x03a\xffb",
                );
                round_trip(
                    $pointer::<Path>::from(PathBuf::from(OsString::from_vec(b"a/\xff".to_vec()))),
                    b"\x03a/\xff",
                );
            }

            #[test]
            fn pointed_values_can_borrow_input_without_extra_bounds() {
                fn decode<'de: 'a, 'a>(input: &'de [u8]) -> $pointer<Borrowed<'a>> {
                    <$pointer<Borrowed<'a>> as Deserialize<'de>>::deserialize(input).unwrap()
                }

                let input = Vec::from(b"\x04\x03abc");
                let value = decode(&input);
                assert_eq!(value.text, "abc");
                assert_eq!(value.text.as_ptr(), input[2..].as_ptr());
                assert_eq!(
                    Serialize::serialize::<DownwardBytes>(&value)
                        .unwrap()
                        .as_ref(),
                    input
                );
            }

            #[test]
            fn generic_recursive_pointers_derive_without_circular_bounds() {
                #[derive(Debug, PartialEq, Serialize, Deserialize)]
                struct Node<T> {
                    #[derse(required)]
                    value: T,
                    next: Option<$pointer<Node<T>>>,
                }

                let value = Node {
                    value: 1u8,
                    next: Some($pointer::new(Node {
                        value: 2,
                        next: None,
                    })),
                };
                round_trip(value, &[5, 1, 1, 2, 2, 0]);
            }

            #[test]
            fn pointers_propagate_read_and_write_errors() {
                assert_eq!(
                    $pointer::<bool>::deserialize(&[2, 99][..]),
                    Err(Error::InvalidBool(2))
                );
                assert_eq!(
                    $pointer::<u16>::deserialize(&[1][..]),
                    Err(Error::DataIsShort {
                        expect: 2,
                        actual: 1,
                    })
                );
                assert_eq!(
                    $pointer::<str>::deserialize(&[1, 255][..]),
                    Err(Error::InvalidString(vec![255]))
                );
                assert!(matches!(
                    $pointer::<CStr>::deserialize(b"\x03abc".as_slice()),
                    Err(Error::InvalidCStr(_))
                ));
                for input in [b"\x80".as_slice(), b"\x03a".as_slice()] {
                    assert!($pointer::<str>::deserialize(input).is_err());
                    assert!($pointer::<[u8]>::deserialize(input).is_err());
                    assert!($pointer::<CStr>::deserialize(input).is_err());
                    assert!($pointer::<OsStr>::deserialize(input).is_err());
                    assert!($pointer::<Path>::deserialize(input).is_err());
                }

                let mut serializer = RejectWrites::default();
                assert_eq!(
                    Serialize::serialize_to(&$pointer::<str>::from("abc"), &mut serializer),
                    Err(Error::InvalidValue("write failed".into()))
                );
                assert_eq!(serializer.0, 1);
            }
        }
    };
}

pointer_tests!(boxed, Box);
pointer_tests!(reference_counted, Rc);
pointer_tests!(atomic_reference_counted, Arc);

#[test]
fn shared_values_decode_into_independent_allocations() {
    let rc = Rc::new(7u8);
    let arc = Arc::new(8u8);
    let value = (rc.clone(), rc, arc.clone(), arc);
    assert!(Rc::ptr_eq(&value.0, &value.1));
    assert!(Arc::ptr_eq(&value.2, &value.3));

    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.as_ref(), &[7, 7, 8, 8]);
    let decoded = <(Rc<u8>, Rc<u8>, Arc<u8>, Arc<u8>)>::deserialize(bytes.as_slice()).unwrap();
    assert_eq!(decoded, value);
    assert!(!Rc::ptr_eq(&decoded.0, &decoded.1));
    assert!(!Arc::ptr_eq(&decoded.2, &decoded.3));
    assert_eq!(Rc::strong_count(&decoded.0), 1);
    assert_eq!(Arc::strong_count(&decoded.2), 1);
}
