fn main() {
    let _ = <[u8; 33] as derse::Deserialize<'_>>::deserialize(&[][..]);
}
