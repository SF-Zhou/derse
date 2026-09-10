use derse::{DownwardBytes, Serialize};

fn encode<T: Serialize>(value: T) -> DownwardBytes {
    value.serialize().unwrap()
}

fn main() {
    encode([0u8; 33]);
}
