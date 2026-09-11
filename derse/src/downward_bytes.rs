use super::{Result, Serializer};
use std::mem::ManuallyDrop;
use std::ptr::{self, NonNull};

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
pub struct DownwardBytes {
    // Owns a Vec-compatible byte allocation, or a dangling pointer at capacity 0.
    // Only the tail [capacity - length, capacity) must be initialized.
    ptr: NonNull<u8>,
    capacity: usize,
    length: usize,
}

// SAFETY: The allocation is exclusively owned and moving it transfers ownership.
unsafe impl Send for DownwardBytes {}
// SAFETY: Shared access exposes only initialized bytes; all writes require &mut self.
unsafe impl Sync for DownwardBytes {}

impl Default for DownwardBytes {
    fn default() -> Self {
        Self {
            ptr: NonNull::dangling(),
            capacity: 0,
            length: 0,
        }
    }
}

impl Drop for DownwardBytes {
    fn drop(&mut self) {
        // SAFETY: The pointer and capacity describe an exclusively owned Vec<u8>
        // allocation (or the empty dangling state). Length 0 exposes no elements
        // and lets Vec free the allocation without reading its uninitialized prefix.
        unsafe { drop(Vec::from_raw_parts(self.ptr.as_ptr(), 0, self.capacity)) };
    }
}

impl DownwardBytes {
    /// Creates an empty buffer without allocating.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty buffer with space for `cap` encoded bytes.
    pub fn with_capacity(cap: usize) -> Self {
        // Reuse Vec's allocation, capacity-overflow and allocation-failure handling.
        // It initializes no bytes. ManuallyDrop transfers the allocation to self.
        let mut allocation = ManuallyDrop::new(Vec::<u8>::with_capacity(cap));
        Self {
            // SAFETY: Vec's pointer is non-null, including at capacity 0.
            ptr: unsafe { NonNull::new_unchecked(allocation.as_mut_ptr()) },
            capacity: allocation.capacity(),
            length: 0,
        }
    }

    /// Returns the number of encoded bytes, excluding unused prefix capacity.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns whether the buffer contains no encoded bytes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the allocation's total capacity, including unused prefix space.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Discards all encoded bytes while retaining the allocation for reuse.
    pub fn clear(&mut self) {
        self.length = 0;
    }

    /// Clears the buffer and reduces its capacity if it exceeds `capacity`.
    ///
    /// A larger requested capacity does not grow the buffer. When shrinking,
    /// the old allocation is replaced with an empty buffer of the requested size.
    pub fn clear_and_shrink_to(&mut self, capacity: usize) {
        if self.capacity() <= capacity {
            self.clear();
        } else {
            *self = Self::with_capacity(capacity);
        }
    }

    /// Borrows the encoded tail in wire order, excluding unused prefix space.
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: The initialized tail lies within the allocation. The pointer is
        // non-null even when empty, and the shared borrow prevents mutation.
        unsafe {
            std::slice::from_raw_parts(
                self.ptr.as_ptr().add(self.capacity - self.length),
                self.length,
            )
        }
    }

    /// Copies `data` before the existing bytes, growing the buffer if necessary.
    ///
    /// The argument's byte order is preserved. The inherent method has no error
    /// result; its [`Serializer`] adapter returns `Ok(())` after this operation.
    pub fn prepend(&mut self, data: impl AsRef<[u8]>) {
        let buf = data.as_ref();
        // Both lengths are at most isize::MAX, so their sum fits in usize.
        self.reserve(self.length + buf.len());
        // Recompute after possible growth to keep less state live across allocation.
        let length = self.length + buf.len();

        // SAFETY: reserve provides enough prefix space. The exclusive borrow of
        // self keeps the destination disjoint from the input. Publish the length
        // only after the copy has initialized the new tail.
        unsafe {
            ptr::copy_nonoverlapping(
                buf.as_ptr(),
                self.ptr.as_ptr().add(self.capacity - length),
                buf.len(),
            );
        }
        self.length = length;
    }

    /// Ensures capacity for at least `size` total encoded bytes.
    ///
    /// `size` includes bytes already present; it is not an additional byte count.
    /// If growth is necessary, capacity at least doubles and existing bytes are
    /// copied to the end of the new allocation.
    pub fn reserve(&mut self, size: usize) {
        if self.capacity() < size {
            self.grow(size);
        }
    }

    // Keep allocation and copying out of prepend loops. Capacity is bounded by
    // isize::MAX, so doubling fits usize; with_capacity checks the resulting size.
    #[cold]
    #[inline(never)]
    fn grow(&mut self, size: usize) {
        let new_cap = std::cmp::max(self.capacity * 2, size);
        let mut new_bytes = Self::with_capacity(new_cap);
        // SAFETY: The source tail is initialized, both ranges fit their distinct
        // allocations, and new_bytes owns its allocation even before it is filled.
        unsafe {
            ptr::copy_nonoverlapping(
                self.ptr.as_ptr().add(self.capacity - self.length),
                new_bytes.ptr.as_ptr().add(new_bytes.capacity - self.length),
                self.length,
            );
        }
        new_bytes.length = self.length;
        *self = new_bytes;
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
