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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{DownwardBytes, Error, Serialize};

    /// A sink that verifies callers stop serializing when a write fails.
    #[derive(Default)]
    pub(crate) struct FailingSerializer {
        pub(crate) writes: usize,
    }

    impl Serializer for FailingSerializer {
        fn prepend(&mut self, _: impl AsRef<[u8]>) -> Result<()> {
            self.writes += 1;
            Err(Error::InvalidValue("write failed".into()))
        }

        fn len(&self) -> usize {
            0
        }
    }

    #[test]
    fn counting_serializer_matches_encoded_length() {
        let mut count = 0usize;
        assert!(Serializer::is_empty(&count));
        count.prepend([]).unwrap();
        assert!(Serializer::is_empty(&count));

        let value = ("hello", vec![1u16, 2, 3]);
        value.serialize_to(&mut count).unwrap();
        let bytes: DownwardBytes = value.serialize().unwrap();
        assert_eq!(Serializer::len(&count), bytes.len());
        assert!(!Serializer::is_empty(&count));

        count.prepend([9, 8]).unwrap();
        assert_eq!(Serializer::len(&count), bytes.len() + 2);

        let mut failing = FailingSerializer::default();
        assert!(failing.is_empty());
        assert_eq!(
            failing.prepend([1]),
            Err(Error::InvalidValue("write failed".into()))
        );
        assert_eq!(failing.writes, 1);
        assert!(failing.is_empty());
    }
}
