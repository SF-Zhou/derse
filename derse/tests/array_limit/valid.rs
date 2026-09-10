use derse::{Deserialize, DownwardBytes, Serialize};

fn encode<T: Serialize>(value: T) -> DownwardBytes {
    value.serialize().unwrap()
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Message {
    value: [u8; 32],
}

fn main() {
    let empty = [0u8; 0].serialize::<DownwardBytes>().unwrap();
    assert!(empty.is_empty());
    assert_eq!(<[u8; 0]>::deserialize(&empty[..]).unwrap(), []);

    let value = [7u8; 32];
    let bytes = value.serialize::<DownwardBytes>().unwrap();
    assert_eq!(&bytes[..], &value);
    assert_eq!(<[u8; 32]>::deserialize(&bytes[..]).unwrap(), value);
    assert_eq!(&encode(value)[..], &value);
    assert_eq!(
        &<[u8; 32] as Serialize>::serialize::<DownwardBytes>(&value).unwrap()[..],
        &value
    );

    let message = Message { value };
    let bytes = encode(&message);
    assert_eq!(Message::deserialize(&bytes[..]).unwrap(), message);
}
