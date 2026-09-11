# Wire format

This document describes the built-in implementations and derive macros in this
checkout. There is no stream header, schema identifier, or protocol version in
the bytes. The application must know which Rust type to decode and how messages
are framed. Crate versions identify releases; they are not embedded wire tags.

All examples show bytes in hexadecimal, in the order a decoder consumes them.
`V(n)` below means the encoding of `VarInt64(n)`, and `||` means concatenation.

## Writing and reading order

`Serializer::prepend` adds bytes before existing output. To produce fields
`a || b`, a serializer writes `b` and then `a`. Bytes within a single write keep
their order. Derived serializers write the body first, measure the increase in
the serializer's length, then prepend that body's byte length.

`Deserializer::pop(n)` consumes `n` bytes. `advance(n)` consumes the same prefix
but returns it as a bounded deserializer of the same type. A derived decoder uses
`advance` to isolate its body, so nested values cannot read past that boundary.
The outer input has already moved past the body even if decoding its fields fails.

## Integers and primitives

| Type | Encoding |
| --- | --- |
| `u8` through `u128`, `i8` through `i128` | Fixed-width little-endian bytes; signed integers use two's complement. |
| `usize`, `isize` | Encoded as `u64`, `i64` respectively, on every target. |
| `f32`, `f64` | IEEE 754 bits in little-endian byte order. |
| `bool` | `00` for false, `01` for true; other values are errors. |
| `char` | Unicode scalar value encoded as `u32`; invalid scalars are errors. |
| `()` and `PhantomData<T>` | No bytes. |

`VarInt64` uses one to ten base-128 digits, with the most significant group first.
Each byte contributes its low seven bits; the high bit indicates that another
byte follows. This is not LEB128. Serialization always emits the shortest form.

| Value | `VarInt64` bytes |
| --- | --- |
| 0 | `00` |
| 127 | `7f` |
| 128 | `81 00` |
| 300 | `82 2c` |
| 16,384 | `81 80 00` |
| `u64::MAX` | `81 ff ff ff ff ff ff ff ff 7f` |

The decoder accepts non-minimal forms, such as `80 00` for zero. It does not
validate overflow bits in a ten-byte value. Ten continuation bytes produce
`Error::VarintIsShort`; exhausting input earlier produces the input reader's
short-data error. Length and collection-count prefixes use this same encoding.

## Strings and byte data

| Type | Encoding |
| --- | --- |
| `str`, `String`, `Cow<str>`, `CompactString` | `V(UTF-8 byte length) || UTF-8 bytes` |
| `[u8]`, `&[u8]`, `Vec<u8>`, `Cow<[u8]>` | `V(byte length) || bytes` |
| `CStr`, `CString` | `V(byte length including NUL) || bytes including the final NUL` |
| `OsStr`, `OsString`, `Path`, `PathBuf` | `V(byte length) || raw Unix path/string bytes` |

String decoders validate UTF-8; C string decoders require one final NUL and no
interior NUL. OS strings and paths do not require UTF-8. For example, `"hi"`
encodes as `02 68 69`, while the C string `"hi\0"` encodes as `03 68 69 00`.

References serialize like their referents. The generic `Cow<T>` implementation
for `T: ToOwned<Owned = T>` also uses `T`'s encoding and decodes to `Cow::Owned`.
Borrowing or ownership is not recorded in the bytes.

## Arrays, tuples, and collections

Tuples of arity 1 through 16 and arrays concatenate their elements in index order
with no prefix. Unit is the zero-element tuple. Arrays support lengths `0..=32`;
longer arrays cannot be deserialized, and serializing them fails at code generation.

The distinction between a byte array and a byte slice is visible on the wire:

| Value | Bytes |
| --- | --- |
| `[1u8, 2, 3]` | `01 02 03` |
| `[1u8, 2, 3].as_slice()` | `03 01 02 03` |
| `vec![1u8, 2, 3]` | `03 01 02 03` |
| `(1u8, 0x0203u16)` | `01 03 02` |

Collections start with `V(element count)`. Maps count entries and encode each
entry as `key || value`. Their iteration choices determine final wire order:

| Collection | Order of elements or entries on the wire |
| --- | --- |
| `Vec`, `VecDeque`, `LinkedList`, `TinyVec` | Logical index/iteration order. |
| `BinaryHeap` | `heap.iter()` order, without sorting. |
| `BTreeSet`, `BTreeMap` | Reverse iteration order, so largest elements/keys first. |
| `HashSet`, `HashMap` | Reverse of their unspecified iteration order. |

Heap iteration is not sorted order. Equal heaps or hash collections need not
serialize to identical bytes. Decoders collect the encoded elements into the
target collection; duplicate set elements and duplicate map keys follow that
collection's `FromIterator` behavior. The count is the number of entries read,
not necessarily the final collection's length.

## Tagged built-in types

| Type | Encoding |
| --- | --- |
| `Option<T>::None` | `00` |
| `Option<T>::Some(value)` | `01 || value` |
| `Result<T, E>::Ok(value)` | `01 || value` |
| `Result<T, E>::Err(error)` | `00 || error` |
| `SocketAddr::V4(value)` | `00 || SocketAddrV4 payload` |
| `SocketAddr::V6(value)` | `01 || SocketAddrV6 payload` |

These tags are bool encodings. These built-in implementations do not use the
string tags or body-length prefixes of derived enums.

| Type | Payload |
| --- | --- |
| `Ipv4Addr`, `Ipv6Addr` | 4 or 16 address octets in network order, without a prefix. |
| `SocketAddrV4` | IPv4 octets, then the port as little-endian `u16`. |
| `SocketAddrV6` | IPv6 octets, port as `u16`, flow info as `u32`, scope ID as `u32`; integers are little-endian. |
| `Duration` | Seconds as little-endian `u64`, then subsecond nanoseconds as little-endian `u32`. |

IP decoders reconstruct address octets independently of host endianness.
`Duration` occupies 12 bytes. Its decoder normalizes nanoseconds outside the
subsecond range, carrying whole seconds into the seconds count. If that addition
overflows, it returns `Error::InvalidValue` with the message `duration overflow`
after consuming both fields. Decoding still reads eight bytes for seconds and
then four bytes for nanoseconds; a short input produces the reader's existing
error without rolling back earlier reads.

## Derived structs and enums

A derived struct encodes as `V(body byte length) || fields`. A derived enum
encodes as `V(body byte length) || variant name string || variant fields`. The
variant name string includes its own byte-length prefix. Rust discriminants,
struct names, and field names are not encoded. A raw variant identifier retains
the spelling used by the macro, including its `r#` prefix.

For example:

```rust
#[derive(derse::Serialize, derse::Deserialize)]
struct Message { id: u16, text: String }

#[derive(derse::Serialize, derse::Deserialize)]
enum Event { Ping, Data(u8) }
```

| Value | Bytes |
| --- | --- |
| `Message { id: 0x1234, text: "hi".into() }` | `05 34 12 02 68 69` |
| `Event::Ping` | `05 04 50 69 6e 67` |
| `Event::Data(7)` | `06 04 44 61 74 61 07` |
| A derived unit struct | `00` |

The body prefix counts bytes, not fields, and excludes its own prefix. Nested
derived values include their own prefixes. A derived unit struct differs from
the primitive `()` encoding.

When the body is empty before a field is read, the default derive policy supplies
`Default::default()`. `#[derse(required)]` always invokes the field decoder;
`#[derse(default = "path")]` calls a zero-argument function instead. Both remove
the field's `Default` requirement. A partially present field is decoded normally
and any error propagates. Unknown bytes after the known fields are skipped.

Appending defaultable trailing fields supports reading older bodies, and the body
boundary lets older readers skip added trailing fields. This does not make field
reordering, type changes, or arbitrary removals compatible. Removing trailing
fields is readable by older schemas only if their missing-field policies permit
it. Renaming a variant changes its tag; adding a variant requires readers that
understand that tag. Field attributes do not alter serialized output.

## Input behavior and limits

`Deserialize::deserialize` permits unused bytes after the decoded value. Use
`deserialize_from` with a retained input cursor when the application needs to
check that no trailing bytes remain. Reads and writes are not transactions:
earlier progress is not rolled back after a later error.

`BytesArray` can borrow when a requested payload fits in its current fragment.
Crossing a fragment boundary, including passing a leading empty fragment for a
non-empty read, produces an owned intermediate buffer. Borrowed targets reject
owned payloads. `Cow<str>` and `Cow<[u8]>` handle either case; `String` and the
owned OS/path/C-string types can also decode fragmented payloads. `CompactString`
currently uses the borrowed string decoder and shares its restriction.

Lengths and counts do not have configurable resource limits. Applications should
bound message sizes and collection counts where necessary. Zero-byte element
types can consume work without consuming payload bytes. `u64` lengths and decoded
`usize`/`isize` values are cast to platform-sized integers without range checks,
so oversized values can truncate on narrower targets. The runtime currently
requires Unix; cross-platform support is not implied by a fixed byte layout.

See [the changelog](../CHANGELOG.md) for compatibility changes since the previous
release, and [the development guide](../CONTRIBUTING.md) for test commands and
buffer memory-safety invariants.
