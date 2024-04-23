use derse::{Derse, DownwardBytes, Serialization};

#[derive(Debug, Derse, PartialEq)]
struct A {
    x: u64,
    y: String,
}

#[test]
fn test_struct() {
    let ser = A {
        x: u64::MAX,
        y: "hello derse!".to_owned(),
    };
    let bytes = ser.serialize::<DownwardBytes>();
    assert_eq!(bytes.len(), 1 + 8 + 1 + 12);

    let der = A::deserialize(&bytes).unwrap();
    assert_eq!(ser, der);
}
