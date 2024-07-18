use super::Result;

/// A trait for serializing data into a byte buffer.
pub trait Serializer {
    /// Prepends data to the buffer.
    ///
    /// # Arguments
    ///
    /// * `data` - The data to prepend.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    fn prepend(&mut self, data: impl AsRef<[u8]>) -> Result<()>;

    /// Returns the length of the serialized data.
    fn len(&self) -> usize;

    /// Checks if the buffer is empty.
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
