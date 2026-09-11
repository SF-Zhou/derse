//! Binary serialization with a buffer that grows from the end toward the front.
//!
//! [`Serialize`] and [`Deserialize`] describe values; [`Serializer`] and
//! [`Deserializer`] provide byte storage and input. Use [`DownwardBytes`] to
//! produce bytes, a `&[u8]` to read contiguous input, or [`BytesArray`] to read
//! several slices as one input.
//!
//! ```
//! # pub use derse::*;
//! use derse::{Deserialize, DownwardBytes, Serialize};
//!
//! #[derive(Debug, PartialEq, Serialize, Deserialize)]
//! struct Message<'a> {
//!     id: u32,
//!     text: &'a str,
//! }
//!
//! # fn main() -> derse::Result<()> {
//! let message = Message { id: 7, text: "hello" };
//! let bytes: DownwardBytes = message.serialize()?;
//! let decoded = Message::deserialize(bytes.as_slice())?;
//! assert_eq!(decoded, message);
//! # Ok(())
//! # }
//! ```
//!
//! # Encoding and supported types
//!
//! Writers **prepend** bytes. Implementations write fields in reverse order so
//! that readers encounter them in declaration order. Length prefixes use
//! [`VarInt64`], whose base-128 digits appear most significant first.
//!
//! - Integers and floating-point numbers use fixed-width little-endian bytes.
//!   `usize` and `isize` use eight bytes, through `u64` and `i64` respectively.
//!   Decoding those types on a narrower target uses a truncating cast.
//! - `bool` is one byte (`0` or `1`); `char` is a `u32` Unicode scalar value.
//!   `()` and `PhantomData<T>` have no bytes.
//! - Strings and byte slices have a byte-length prefix. `CStr` and `CString`
//!   include the final NUL in their payload and length. `OsStr`, `OsString`,
//!   `Path`, and `PathBuf` preserve Unix bytes without UTF-8 validation.
//! - Arrays `[T; N]` for `N` in `0..=32` and tuples of up to 16 fields contain
//!   their elements in order, with no prefix. Serializing an array longer than
//!   32 fails during code generation (`cargo build`); `cargo check` alone does
//!   not evaluate the assertion. Deserialization constructs arrays directly.
//!   An array such as `[u8; 3]` therefore differs from the slice `&[u8]`.
//! - `Vec`, `VecDeque`, `LinkedList`, `BinaryHeap`, sets, and maps begin with an
//!   element count. Maps encode each key before its value. Vectors and lists
//!   preserve element order; heaps preserve `heap.iter()` order, which is not
//!   sorted order. B-tree collections encode the largest key first. Hash
//!   collection bytes depend on their iteration order.
//! - `Option<T>` begins with `0` for `None` or `1` followed by the `Some` value.
//!   `Result<T, E>` uses `1` before an `Ok` value and `0` before an `Err` value.
//!   IP addresses, socket addresses, and `Duration` are also supported.
//!
//! # Derived types and schema changes
//!
//! Derives support structs and enums, including generic and borrowed fields.
//! Each derived value starts with its body's byte length. Struct fields follow
//! in declaration order; enum bodies start with a length-prefixed variant name,
//! followed by that variant's fields. Field names are not encoded.
//!
//! A reader confines field decoding to the declared body. It skips trailing
//! bytes that its schema does not use, and uses `Default::default()` for fields
//! reached after the body is exhausted. A partially present field still fails
//! to decode. This permits appending fields when their defaults are suitable;
//! reordering fields, changing their encodings, or renaming an enum variant can
//! change compatibility.
//!
//! Field attributes select a missing-field policy:
//!
//! - `#[derse(required)]` always invokes the field decoder.
//! - `#[derse(default)]` explicitly uses the usual `Default` fallback.
//! - `#[derse(default = "path::to_function")]` calls that zero-argument function
//!   instead of requiring `Default`.
//!
//! These attributes belong on fields, including tuple fields. Derived bounds
//! follow field types: for example, `PhantomData<T>` does not require `T` to
//! implement the serialization traits.
//!
//! `#[derse(recursive)]` marks generic recursion hidden behind a type alias or
//! qualified path. It skips inferred serialization and deserialization bounds
//! for the whole field; supply any required generic bounds explicitly. It can
//! accompany a missing-field policy and does not remove its `Default` requirement.
//! Recursion written as `Self` or the unqualified type name is detected automatically.
//!
//! # Borrowing and input consumption
//!
//! Borrowed outputs such as `&str` and `&[u8]` need a payload that the input can
//! return as one borrowed slice. With fragmented input, use `String`, `Vec<u8>`,
//! `Cow<str>`, or `Cow<[u8]>` when a payload may cross slice boundaries.
//! [`BytesArray`] documents when a read borrows or allocates.
//!
//! [`Deserialize::deserialize`] reads one value and does not reject unused
//! input. Use [`Deserialize::deserialize_from`] to inspect or reuse the
//! remaining input. Reading and writing are not transactional: an error can
//! leave input consumed or output partially written.
//!
//! # Features and platforms
//!
//! The `compact_str` feature adds `CompactString` support, and `tinyvec` adds
//! `TinyVec` support. `full` enables both. `CompactString` currently decodes
//! through `&str`, so its payload must be borrowable even though the output is
//! owned. `TinyVec` uses the same count-prefixed encoding as `Vec`.
//!
//! The crate requires `std` and currently targets Unix: its operating-system
//! string implementation unconditionally uses `std::os::unix`.

mod bytes_array;
mod deserializer;
mod downward_bytes;
mod error;
mod impls;
mod serializer;
mod varint64;

pub use bytes_array::BytesArray;
pub use deserializer::Deserializer;
pub use downward_bytes::DownwardBytes;
pub use error::{Error, Result};
pub use serializer::Serializer;
pub use varint64::VarInt64;

pub use derse_derive::{Deserialize, Serialize};

/// Encodes a value by prepending its bytes to a [`Serializer`].
///
/// For a value encoded as `first` followed by `second`, write `second` first.
/// Derived implementations apply this ordering and add their body-length prefix.
pub trait Serialize {
    /// Encodes into a default-constructed writer and returns that writer.
    ///
    /// Choose [`DownwardBytes`] for bytes or `usize` to count the encoded length.
    /// The counting writer still visits every value and propagates its errors.
    fn serialize<S: Serializer + Default>(&self) -> Result<S> {
        let mut serializer = S::default();
        self.serialize_to(&mut serializer)?;
        Ok(serializer)
    }

    /// Prepends this value's complete encoding to an existing writer.
    ///
    /// Existing bytes follow the new value. If encoding fails, the writer can
    /// contain a partial encoding; implementations do not roll back earlier writes.
    fn serialize_to<S: Serializer>(&self, serializer: &mut S) -> Result<()>;
}

/// Decodes a value from the front of a [`Deserializer`].
///
/// `'a` is the lifetime of bytes that the input can lend to the result. Owned
/// outputs need not borrow, and borrowed output lifetimes may be shorter than
/// `'a`. The input and the requested type together determine the wire format;
/// it is not self-describing.
pub trait Deserialize<'a> {
    /// Decodes one value, taking ownership of the input cursor.
    ///
    /// Trailing bytes are allowed and the remaining cursor is discarded. Use
    /// [`deserialize_from`](Self::deserialize_from) to check for trailing bytes.
    fn deserialize<D: Deserializer<'a>>(mut der: D) -> Result<Self>
    where
        Self: Sized,
    {
        Self::deserialize_from(&mut der)
    }

    /// Decodes one value and advances the caller's cursor past its encoding.
    ///
    /// An error can leave the cursor partially consumed. For derived values,
    /// the entire length-delimited body is removed from the outer cursor before
    /// its fields are decoded, including when a field subsequently fails.
    ///
    /// ```
    /// use derse::Deserialize;
    ///
    /// let mut input = &[7, 0, 9][..];
    /// assert_eq!(u16::deserialize_from(&mut input)?, 7);
    /// assert_eq!(input, &[9]);
    /// # Ok::<(), derse::Error>(())
    /// ```
    fn deserialize_from<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized;
}

/// Separates the length prefix and body decoder generated by `Deserialize`.
///
/// Normal callers should use [`Deserialize`]. This trait exposes the two steps
/// for callers that need to delimit or inspect a derived value's body themselves.
/// Call [`Deserializer::advance`] with the decoded length before decoding fields.
pub trait DetailedDeserialize<'a> {
    /// Reads the [`VarInt64`] body length, leaving the body in the input.
    ///
    /// The returned count excludes the length prefix. This does not check that
    /// the body is present; [`Deserializer::advance`] performs that check.
    fn deserialize_len<D: Deserializer<'a>>(buf: &mut D) -> Result<usize>;

    /// Decodes fields from an input already limited to one value's body.
    ///
    /// No outer length prefix is read. Enum bodies still include their variant
    /// name. Missing-field policies observe this cursor's emptiness, and any
    /// fields unknown to this schema remain in the body cursor afterward.
    fn deserialize_fields<D: Deserializer<'a>>(buf: &mut D) -> Result<Self>
    where
        Self: Sized;
}
