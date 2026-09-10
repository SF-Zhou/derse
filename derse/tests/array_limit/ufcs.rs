use derse::{DownwardBytes, Serialize};

fn main() {
    <[u8; 33] as Serialize>::serialize::<DownwardBytes>(&[0; 33]).unwrap();
}
