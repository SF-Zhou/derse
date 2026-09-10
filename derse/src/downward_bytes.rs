use super::{Result, Serializer};

/// A reusable byte buffer optimized for prepending an encoding.
///
/// Written bytes occupy the end of the allocation. A prepend copies new bytes
/// into the space immediately before them; existing bytes move only when the
/// allocation grows. [`as_slice`](Self::as_slice) and `Deref<Target = [u8]>`
/// expose the encoded bytes in their final order.
///
/// ```
/// use derse::DownwardBytes;
///
/// let mut bytes = DownwardBytes::with_capacity(16);
/// bytes.prepend("world");
/// bytes.prepend("hello ");
/// assert_eq!(bytes.as_slice(), b"hello world");
/// bytes.clear();
/// assert!(bytes.is_empty());
/// assert_eq!(bytes.capacity(), 16);
/// ```
#[derive(Default)]
pub struct DownwardBytes(Vec<u8>);

impl DownwardBytes {
    /// Creates an empty buffer without allocating.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty buffer with space for `cap` encoded bytes.
    pub fn with_capacity(cap: usize) -> Self {
        Self(Self::new_vec(cap, cap))
    }

    // The backing Vec's length is used as the start offset of the encoded tail,
    // rather than the number of encoded bytes. Do not expose the Vec as data.
    fn offset(&self) -> usize {
        self.0.len()
    }

    /// Returns the number of encoded bytes, excluding unused prefix capacity.
    pub fn len(&self) -> usize {
        self.capacity() - self.offset()
    }

    /// Returns whether the buffer contains no encoded bytes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the allocation's total capacity, including unused prefix space.
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Discards all encoded bytes while retaining the allocation for reuse.
    pub fn clear(&mut self) {
        unsafe { self.0.set_len(self.capacity()) };
    }

    /// Clears the buffer and reduces its capacity if it exceeds `capacity`.
    ///
    /// A larger requested capacity does not grow the buffer. When shrinking,
    /// the old allocation is replaced with an empty buffer of the requested size.
    pub fn clear_and_shrink_to(&mut self, capacity: usize) {
        if self.capacity() <= capacity {
            unsafe { self.0.set_len(self.capacity()) };
        } else {
            *self = Self::with_capacity(capacity);
        }
    }

    /// Borrows the encoded tail in wire order, excluding unused prefix space.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.0.as_ptr().byte_add(self.offset()), self.len()) }
    }

    // Returns the destination tail used when moving bytes to a new allocation.
    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.0.as_mut_ptr().byte_add(self.offset()), self.len())
        }
    }

    /// Copies `data` before the existing bytes, growing the buffer if necessary.
    ///
    /// The argument's byte order is preserved. The inherent method has no error
    /// result; its [`Serializer`] adapter returns `Ok(())` after this operation.
    pub fn prepend(&mut self, data: impl AsRef<[u8]>) {
        let buf = data.as_ref();
        if self.offset() < buf.len() {
            self.reserve(self.len() + buf.len());
        }

        let new_offset = self.offset() - buf.len();
        self.0[new_offset..].copy_from_slice(buf);
        self.0.truncate(new_offset)
    }

    /// Ensures capacity for at least `size` total encoded bytes.
    ///
    /// `size` includes bytes already present; it is not an additional byte count.
    /// If growth is necessary, capacity at least doubles and existing bytes are
    /// copied to the end of the new allocation.
    pub fn reserve(&mut self, size: usize) {
        if self.capacity() < size {
            let new_cap = std::cmp::max(self.capacity() * 2, size);
            let mut new_bytes = Self(Self::new_vec(new_cap, new_cap - self.len()));
            new_bytes.as_mut_slice().copy_from_slice(self.as_ref());
            self.0 = new_bytes.0;
        }
    }

    // Allocates the backing storage and sets its encoded-tail offset. This
    // representation uses Vec length as bookkeeping, not as an initialized-data
    // count; callers must not read the unused prefix as encoded bytes.
    #[allow(clippy::uninit_vec)]
    fn new_vec(cap: usize, len: usize) -> Vec<u8> {
        let mut vec = Vec::with_capacity(cap);
        unsafe { vec.set_len(len) };
        vec
    }
}

impl Serializer for DownwardBytes {
    fn prepend(&mut self, data: impl AsRef<[u8]>) -> Result<()> {
        self.prepend(data);
        Ok(())
    }

    fn len(&self) -> usize {
        self.len()
    }
}

impl PartialEq for DownwardBytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl std::fmt::Debug for DownwardBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DownwardBytes({:?})", self.as_slice())
    }
}

impl std::ops::Deref for DownwardBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_downward_bytes_create() {
        assert_eq!(DownwardBytes::new().capacity(), 0);

        let mut bytes = DownwardBytes::with_capacity(8);
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), 8);

        bytes.prepend("world!");
        assert_eq!(bytes.len(), 6);
        assert_eq!(bytes.as_slice(), b"world!");

        bytes.prepend("hello ");
        assert_eq!(bytes.len(), 12);
        assert_eq!(bytes.as_slice(), b"hello world!");
    }

    #[test]
    fn test_downward_bytes_prepend() {
        let mut bytes = DownwardBytes::new();
        assert!(bytes.is_empty());
        assert_eq!(bytes.len(), 0);
        assert_eq!(bytes.capacity(), 0);
        assert_eq!(format!("{:?}", bytes), "DownwardBytes([])");

        const N: usize = 100000;
        for i in 0..N {
            bytes.prepend([i as u8]);
        }

        bytes
            .as_ref()
            .iter()
            .rev()
            .enumerate()
            .for_each(|(idx, &value)| {
                assert_eq!(idx as u8, value);
            });

        assert_eq!(bytes.len(), N);
        assert_eq!(bytes.capacity(), N.next_power_of_two());

        assert!(!Serializer::is_empty(&bytes));

        bytes.clear();
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), N.next_power_of_two());

        bytes.clear_and_shrink_to(N.next_power_of_two());
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), N.next_power_of_two());

        bytes.clear_and_shrink_to(N);
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), N);

        bytes.clear_and_shrink_to(N.next_power_of_two());
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), N);
    }
}
