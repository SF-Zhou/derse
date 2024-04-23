use super::Serializer;

#[derive(Default)]
pub struct DownwardBytes(Vec<u8>);

impl DownwardBytes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self(Self::new_vec(cap, cap))
    }

    fn offset(&self) -> usize {
        self.0.len()
    }

    pub fn len(&self) -> usize {
        self.capacity() - self.offset()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.0.as_ptr().byte_add(self.offset()), self.len()) }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(self.0.as_mut_ptr().byte_add(self.offset()), self.len())
        }
    }

    pub fn prepend(&mut self, data: impl AsRef<[u8]>) {
        let buf = data.as_ref();
        if self.offset() < buf.len() {
            self.reserve(self.len() + buf.len());
        }

        let new_offset = self.offset() - buf.len();
        self.0[new_offset..].copy_from_slice(buf);
        self.0.truncate(new_offset)
    }

    pub fn reserve(&mut self, size: usize) {
        if self.capacity() < size {
            let new_cap = std::cmp::max(self.capacity() * 2, size);
            let mut new_bytes = Self(Self::new_vec(new_cap, new_cap - self.len()));
            new_bytes.as_mut_slice().copy_from_slice(self.as_ref());
            self.0 = new_bytes.0;
        }
    }

    #[allow(clippy::uninit_vec)]
    fn new_vec(cap: usize, len: usize) -> Vec<u8> {
        let mut vec = Vec::with_capacity(cap);
        unsafe { vec.set_len(len) };
        vec
    }
}

impl Serializer for DownwardBytes {
    fn prepend(&mut self, data: impl AsRef<[u8]>) {
        self.prepend(data)
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
    #[test]
    fn test_downward_bytes_create() {
        use super::DownwardBytes;

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
        use super::*;

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

        fn to_serializer(serializer: &impl Serializer) {
            assert!(!serializer.is_empty());
        }
        to_serializer(&bytes);
    }
}
