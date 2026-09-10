use derse::{DownwardBytes, Serialize};

fn main() {
    [0u8; 33].serialize::<DownwardBytes>().unwrap();
}
