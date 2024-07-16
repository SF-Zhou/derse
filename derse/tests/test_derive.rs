use std::{borrow::Cow, marker::PhantomData};

use derse::{BytesArray, Deserialize, Deserializer, DetailedDeserialize, DownwardBytes, Serialize};

#[test]
fn test_named_struct() {
    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
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
    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
    struct A(u32, u64, String);

    let ser = A(u32::MAX, u64::MAX, "hello derse!".to_owned());
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.len(), 1 + 4 + 8 + 1 + 12);

    let der = A::deserialize(&bytes[..]).unwrap();
    assert_eq!(ser, der);
}

#[test]
fn test_unit_struct() {
    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
    struct A;

    let ser = A;
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    assert_eq!(bytes.len(), 1);

    let der = A::deserialize(&bytes[..]).unwrap();
    assert_eq!(ser, der);
}

#[test]
fn test_compatibility() {
    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
    struct A1(String, u64);

    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
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
    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
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
    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
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
    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
    struct V1 {
        x: u64,
    }

    #[derive(Debug, derse::deserialize, derse::serialize, PartialEq)]
    struct V2 {
        x: u64,
        y: String,
    }

    let ser = V2 {
        x: 233,
        y: String::from("hello"),
    };
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    let mut buf = &bytes[..];

    let len = V1::deserialize_len(&mut buf).unwrap();
    let mut buf = buf.advance(len).unwrap();
    let der = V1::deserialize_fields(&mut buf).unwrap();
    assert_eq!(der.x, ser.x);
    let der = String::deserialize(buf).unwrap();
    assert_eq!(der, ser.y);
}

#[test]
fn test_enum() {
    #[derive(Debug, derse::serialize, derse::deserialize, PartialEq)]
    enum Demo {
        A,
        B(i32),
        C { x: i32, y: String },
    }

    let ser = Demo::A;
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    let ty = <&str>::deserialize(&bytes[1..]).unwrap();
    assert_eq!(ty, "A");
    let der = Demo::deserialize(&bytes[..]).unwrap();
    assert_eq!(ser, der);

    let ser = Demo::B(233);
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    let ty = <&str>::deserialize(&bytes[1..]).unwrap();
    assert_eq!(ty, "B");
    let value = i32::deserialize(&bytes[3..]).unwrap();
    assert_eq!(value, 233);
    let der = Demo::deserialize(&bytes[..]).unwrap();
    assert_eq!(ser, der);

    let ser = Demo::C {
        x: 233,
        y: "hello".into(),
    };
    let bytes = ser.serialize::<DownwardBytes>().unwrap();
    let ty = <&str>::deserialize(&bytes[1..]).unwrap();
    assert_eq!(ty, "C");
    let value = i32::deserialize(&bytes[3..]).unwrap();
    assert_eq!(value, 233);
    let str = <&str>::deserialize(&bytes[7..]).unwrap();
    assert_eq!(str, "hello");
    let der = Demo::deserialize(&bytes[..]).unwrap();
    assert_eq!(ser, der);

    let mut bytes = "D".serialize::<DownwardBytes>().unwrap();
    2u8.serialize_to(&mut bytes).unwrap();
    println!("{}", Demo::deserialize(&bytes[..]).unwrap_err());
}
