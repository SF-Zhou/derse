use derse::DownwardBytes;
use std::panic::{catch_unwind, AssertUnwindSafe, RefUnwindSafe, UnwindSafe};

#[test]
fn empty_buffers_and_growth_preserve_capacity_rules() {
    for capacity in [0, 1, 16] {
        let mut bytes = DownwardBytes::with_capacity(capacity);
        bytes.prepend([]);
        bytes.reserve(0);
        assert_eq!(bytes.as_slice(), b"");
        assert!(bytes.is_empty());
        assert_eq!(bytes.capacity(), capacity);

        bytes.clear_and_shrink_to(0);
        assert_eq!(bytes.capacity(), 0);
        assert_eq!(bytes.as_slice(), b"");
        bytes.prepend([]);
        assert_eq!(bytes.capacity(), 0);
        bytes.prepend(b"a");
        assert_eq!(bytes.capacity(), 1);
        bytes.prepend(b"b");
        assert_eq!(bytes.capacity(), 2);
        bytes.reserve(3);
        assert_eq!(bytes.capacity(), 4);
        bytes.reserve(9);
        assert_eq!(bytes.capacity(), 9);
        assert_eq!(bytes.as_slice(), b"ba");
    }
}

#[test]
fn partially_written_buffers_survive_growth_clear_and_shrink() {
    let mut bytes = DownwardBytes::with_capacity(13);
    bytes.prepend(b"tail");
    bytes.reserve(64);
    assert_eq!(bytes.capacity(), 64);
    assert_eq!(bytes.as_slice(), b"tail");
    bytes.prepend(b"head ");
    assert_eq!(bytes.as_slice(), b"head tail");

    bytes.clear();
    assert!(bytes.is_empty());
    assert_eq!(bytes.capacity(), 64);
    bytes.prepend(b"new");
    assert_eq!(bytes.as_slice(), b"new");

    bytes.clear_and_shrink_to(7);
    assert_eq!(bytes.capacity(), 7);
    assert!(bytes.is_empty());
    bytes.prepend(b"refill");
    assert_eq!(bytes.as_slice(), b"refill");
    bytes.clear_and_shrink_to(99);
    assert_eq!(bytes.capacity(), 7);
    assert!(bytes.is_empty());
    bytes.prepend(b"x");
    bytes.clear_and_shrink_to(7);
    assert!(bytes.is_empty());
    assert_eq!(bytes.capacity(), 7);
}

#[test]
fn prepends_without_growth_keep_existing_bytes_at_the_same_address() {
    let mut bytes = DownwardBytes::with_capacity(16);
    bytes.prepend(b"tail");
    let tail = bytes.as_slice().as_ptr();
    bytes.prepend(b"head ");
    assert_eq!(bytes.as_slice(), b"head tail");
    assert_eq!(bytes.as_slice()[5..].as_ptr(), tail);

    let start = bytes.as_slice().as_ptr();
    bytes.prepend([]);
    bytes.reserve(bytes.len());
    bytes.reserve(bytes.capacity());
    assert_eq!(bytes.as_slice().as_ptr(), start);
    assert_eq!(bytes.capacity(), 16);

    bytes.clear();
    bytes.prepend(b"next");
    assert_eq!(bytes.as_slice().as_ptr(), tail);
    assert_eq!(bytes.as_slice(), b"next");
}

#[test]
fn mixed_operations_match_a_vec_reference() {
    let mut bytes = DownwardBytes::new();
    let mut expected = Vec::new();
    let mut capacity = 0;
    let mut state = 0x8a5c_31e7_u32;

    for step in 0..192 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        match (state >> 16) % 4 {
            0 => {
                let data = [step as u8, 0, 255, (state >> 24) as u8];
                let data = &data[..(state as usize % 5)];
                let required = expected.len() + data.len();
                if required > capacity {
                    capacity = required.max(capacity * 2);
                }
                bytes.prepend(data);
                expected.splice(..0, data.iter().copied());
            }
            1 => {
                let size = state as usize % 64;
                if size > capacity {
                    capacity = size.max(capacity * 2);
                }
                bytes.reserve(size);
            }
            2 => {
                bytes.clear();
                expected.clear();
            }
            _ => {
                let size = state as usize % 32;
                bytes.clear_and_shrink_to(size);
                capacity = capacity.min(size);
                expected.clear();
            }
        }
        assert_eq!(bytes.as_slice(), expected, "operation {step}");
        assert_eq!(bytes.len(), expected.len(), "operation {step}");
        assert_eq!(bytes.is_empty(), expected.is_empty(), "operation {step}");
        assert_eq!(bytes.capacity(), capacity, "operation {step}");
    }
}

#[test]
fn capacity_overflow_panics_without_changing_existing_data() {
    let invalid_capacity = isize::MAX as usize + 1;
    assert!(catch_unwind(|| DownwardBytes::with_capacity(invalid_capacity)).is_err());

    let mut bytes = DownwardBytes::with_capacity(8);
    bytes.prepend(b"tail");
    let start = bytes.as_slice().as_ptr();
    for requested in [invalid_capacity, usize::MAX] {
        assert!(catch_unwind(AssertUnwindSafe(|| bytes.reserve(requested))).is_err());
        assert_eq!(bytes.as_slice(), b"tail");
        assert_eq!(bytes.capacity(), 8);
        assert_eq!(bytes.as_slice().as_ptr(), start);
    }
    bytes.prepend(b"head");
    assert_eq!(bytes.as_slice(), b"headtail");
}

#[test]
fn buffer_keeps_vec_size_and_auto_traits() {
    fn assert_traits<T: Send + Sync + Unpin + UnwindSafe + RefUnwindSafe>() {}
    assert_traits::<DownwardBytes>();
    assert_eq!(
        std::mem::size_of::<DownwardBytes>(),
        std::mem::size_of::<Vec<u8>>()
    );
    assert_eq!(
        std::mem::align_of::<DownwardBytes>(),
        std::mem::align_of::<Vec<u8>>()
    );
}
