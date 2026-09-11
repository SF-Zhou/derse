use super::Result;

/// A destination that accepts bytes at the front of its current output.
///
/// Each [`prepend`](Self::prepend) preserves the order of its argument's bytes
/// and places them before all previous writes. [`len`](Self::len) reports the
/// total encoded byte count, including for destinations that only count bytes.
///
/// [`DownwardBytes`](crate::DownwardBytes) stores the output. `usize` implements
/// this trait as a counter, allowing [`Serialize::serialize`](crate::Serialize::serialize)
/// to measure an encoding without allocating an output buffer:
///
/// ```
/// use derse::{DownwardBytes, Serialize};
///
/// let value = (7u16, "hello");
/// let length: usize = value.serialize()?;
/// let bytes: DownwardBytes = value.serialize()?;
/// assert_eq!(length, bytes.len());
/// # Ok::<(), derse::Error>(())
/// ```
pub trait Serializer {
    /// Places `data` before the existing output and increases its length.
    ///
    /// The data is consumed during this call; the writer cannot retain the
    /// borrowed argument. Custom writers may report a write error. No rollback
    /// guarantee is imposed on a failed write.
    fn prepend(&mut self, data: impl AsRef<[u8]>) -> Result<()>;

    /// Returns the number of bytes written or counted so far.
    fn len(&self) -> usize;

    /// Returns whether the writer contains zero encoded bytes.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Counts encoded bytes without retaining them.
///
/// Start at zero for the length of one value. Addition has the usual `usize`
/// overflow behavior; this writer does not impose an output-size limit.
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
