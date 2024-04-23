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
