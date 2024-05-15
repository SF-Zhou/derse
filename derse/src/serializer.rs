use super::Result;

pub trait Serializer {
    fn prepend(&mut self, data: impl AsRef<[u8]>) -> Result<()>;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Serializer for usize {
    fn prepend(&mut self, data: impl AsRef<[u8]>) -> Result<()> {
        *self += data.as_ref().len();
        Ok(())
    }

    fn len(&self) -> usize {
        *self
    }
}
