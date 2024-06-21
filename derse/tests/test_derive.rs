use std::{borrow::Cow, marker::PhantomData};

use derse::{BytesArray, Derse, Deserialize, DownwardBytes, Serialize};

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
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.len(), 1 + 8 + 1 + 12);

    assert_eq!(ser.serialize::<usize>().unwrap(), bytes.len());

    let der = A::deserialize(&bytes[..]).unwrap();
    assert_eq!(ser, der);
}

#[test]
fn test_unnamed_struct() {
    #[derive(Debug, Derse, PartialEq)]
    struct A(u32, u64, String);

    let ser = A(u32::MAX, u64::MAX, "hello derse!".to_owned());
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.len(), 1 + 4 + 8 + 1 + 12);

    let der = A::deserialize(&bytes[..]).unwrap();
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
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 1 + 1 + 12 + 8);

        let der = A2::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser.0, der.0);
        assert_eq!(ser.1, der.1);
        assert!(der.2.is_empty());
    }

    {
        let ser = A2("hello derse!".to_owned(), 12138, "more data".to_owned());
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 1 + 1 + 12 + 8 + 1 + 9);

        let der = A2::deserialize(&bytes[..]).unwrap();
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
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 1 + 1 + 12);

        let der = A::deserialize(&bytes[..]).unwrap();
        assert!(matches!(der.0, Cow::Borrowed(_)));
        assert_eq!(ser.0, der.0);

        let c = [bytes.as_ref()];
        let mut vec = BytesArray::new(&c);
        let der = A::deserialize_from(&mut vec).unwrap();
        assert!(matches!(der.0, Cow::Borrowed(_)));
        assert_eq!(ser.0, der.0);
    }
}

#[test]
fn test_struct_with_generic() {
    #[derive(Debug, Derse, PartialEq)]
    struct A<'a, S: Default + Serialize + Deserialize<'a>>(i32, S, PhantomData<&'a ()>);

    {
        let ser = A(233, "hello".to_string(), Default::default());
        let bytes = ser.serialize::<DownwardBytes>().unwrap();
        assert_eq!(bytes.len(), 1 + 4 + 1 + 5);

        let der = A::<String>::deserialize(&bytes[..]).unwrap();
        assert_eq!(ser.0, der.0);
        assert_eq!(ser.1, der.1);

        let c = [bytes.as_ref()];
        let mut vec = BytesArray::new(&c);
        let der = A::<&str>::deserialize_from(&mut vec).unwrap();
        assert_eq!(ser.0, der.0);
        assert_eq!(ser.1, der.1);
    }
}

#[test]
fn test_struct_with_remain_buf() {
    #[derive(Debug, Derse, PartialEq)]
    struct V1 {
        x: u64,
    }

    #[derive(Debug, Derse, PartialEq)]
    struct V2 {
        x: u64,
        y: String,
    }

    let ser = V2 {
        x: 233,
        y: String::from("hello"),
    };
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    let buf = &bytes[..];
    let (der, remain) = V1::deserialize_and_split(buf).unwrap();
    assert_eq!(der.x, ser.x);
    let der = String::deserialize(remain).unwrap();
    assert_eq!(der, ser.y);
}
