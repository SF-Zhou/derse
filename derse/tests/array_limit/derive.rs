use derse::{DownwardBytes, Serialize};

#[derive(Serialize)]
struct Message {
    value: [u8; 33],
}

fn main() {
    Message { value: [0; 33] }
        .serialize::<DownwardBytes>()
        .unwrap();
}
