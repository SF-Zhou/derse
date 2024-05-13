use std::borrow::Cow;

use derse::{Derse, DownwardBytes, Serialization};

#[test]
fn test_named_struct() {
    #[derive(Debug, Derse, PartialEq)]
    struct A {
        x: u64,
        y: String,
    }

    let ser = A {
        x: u64::MAX,
        y: "hello derse!".to_owned(),
    };
    let bytes = ser.serialize::<DownwardBytes>();
    assert_eq!(bytes.len(), 1 + 8 + 1 + 12);

    let der = A::deserialize(&bytes).unwrap();
    assert_eq!(ser, der);
}

#[test]
fn test_unnamed_struct() {
    #[derive(Debug, Derse, PartialEq)]
    struct A(u32, u64, String);

    let ser = A(u32::MAX, u64::MAX, "hello derse!".to_owned());
    let bytes = ser.serialize::<DownwardBytes>();
    assert_eq!(bytes.len(), 1 + 4 + 8 + 1 + 12);

    let der = A::deserialize(&bytes).unwrap();
    assert_eq!(ser, der);
}

#[test]
fn test_compatibility() {
    #[derive(Debug, Derse, PartialEq)]
    struct A1(String, u64);

    #[derive(Debug, Derse, PartialEq)]
    struct A2(String, u64, String); // add a field.

    {
        let ser = A1("hello derse!".to_owned(), 12138);
        let bytes = ser.serialize::<DownwardBytes>();
        assert_eq!(bytes.len(), 1 + 1 + 12 + 8);

        let der = A2::deserialize(&bytes).unwrap();
        assert_eq!(ser.0, der.0);
        assert_eq!(ser.1, der.1);
        assert!(der.2.is_empty());
    }

    {
        let ser = A2("hello derse!".to_owned(), 12138, "more data".to_owned());
        let bytes = ser.serialize::<DownwardBytes>();
        assert_eq!(bytes.len(), 1 + 1 + 12 + 8 + 1 + 9);

        let der = A2::deserialize(&bytes).unwrap();
        assert_eq!(ser.0, der.0);
        assert_eq!(ser.1, der.1);
    }
}

#[test]
fn test_struct_with_lifetime() {
    #[derive(Debug, Derse, PartialEq)]
    struct A<'a>(Cow<'a, str>);

    {
        let ser = A(Cow::Owned("hello derse!".to_owned()));
        let bytes = ser.serialize::<DownwardBytes>();
        assert_eq!(bytes.len(), 1 + 1 + 12);

        let der = A::deserialize(&bytes).unwrap();
        assert!(matches!(der.0, Cow::Borrowed(_)));
        assert_eq!(ser.0, der.0);
    }
}
