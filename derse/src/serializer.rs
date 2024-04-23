pub trait Serializer {
    fn prepend(&mut self, data: impl AsRef<[u8]>);

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
