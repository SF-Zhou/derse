# derse

[![Rust](https://github.com/SF-Zhou/derse/actions/workflows/rust.yml/badge.svg)](https://github.com/SF-Zhou/derse/actions/workflows/rust.yml)
[![Crates.io Version](https://img.shields.io/crates/v/derse)](https://crates.io/crates/derse)
[![codecov](https://codecov.io/gh/SF-Zhou/derse/graph/badge.svg?token=8I6CQT5VJ5)](https://codecov.io/gh/SF-Zhou/derse)
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FSF-Zhou%2Fderse.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2FSF-Zhou%2Fderse?ref=badge_shield)

derse is a binary serialization library for Rust. It provides derive macros,
a buffer that grows by prepending bytes, and deserializers for contiguous or
fragmented input. Derived structs and enums have length-delimited bodies so
readers can skip added trailing fields.

The runtime requires `std` and currently builds on Unix targets. It is not a
Serde format: types implement derse's own `Serialize` and `Deserialize` traits.

## Installation

The version in this checkout is a prerelease target. Once it is published, use:

```toml
[dependencies]
derse = "=0.2.0-alpha"
```

The exact requirement keeps testing on that prerelease. The runtime re-exports
both derive macros and depends on the matching `derse-derive` version; consumers
normally need only the `derse` dependency. See the
[release guide](https://github.com/SF-Zhou/derse/blob/main/docs/releasing.md)
for version policy and stable-release upgrades.

## Serialize and deserialize

```rust
use derse::{Deserialize, DownwardBytes, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Message {
    id: u32,
    text: String,
}

let message = Message { id: 7, text: "hello".into() };
let bytes = message.serialize::<DownwardBytes>().unwrap();
let decoded = Message::deserialize(bytes.as_slice()).unwrap();
assert_eq!(decoded, message);

// A usize serializer counts the encoded bytes without allocating a buffer.
let size = message.serialize::<usize>().unwrap();
assert_eq!(size, bytes.len());
```

`Serializer::prepend` puts each write before the existing output. Manual
implementations therefore write the last field first. `serialize_to` can reuse
a buffer; call `clear()` first when starting an independent message.

`deserialize` reads one value and allows unused input after it. To read several
values or check for trailing bytes, keep a mutable input slice and use
`deserialize_from`.

## Borrowed and fragmented input

`&str`, `&[u8]`, `&CStr`, `&OsStr`, and `&Path` borrow their payload from the input.
Use `BytesArray` to read a list of byte slices without concatenating the entire
message first. A read contained in its current fragment can borrow; a read
crossing fragments allocates an owned buffer.

```rust
use derse::{BytesArray, Deserialize};
use std::borrow::Cow;

let fragments: &[&[u8]] = &[b"\x05he", b"llo"];
let text = Cow::<str>::deserialize(BytesArray::new(fragments)).unwrap();
assert_eq!(text, "hello");
assert!(matches!(text, Cow::Owned(_)));
```

A borrowed target cannot accept an owned intermediate payload and returns an
error in that case. `String`, `Vec<u8>`, `Cow<str>`, and `Cow<[u8]>` can accept
fragmented payloads. `CompactString` currently decodes through `&str`, so its
payload must be borrowable too.

## Derive and schema changes

The macros support named, tuple, and unit structs and enum variants, including
generic and borrowed fields. They infer field trait bounds while respecting
existing bounds. A `PhantomData<T>` field does not require `T` to be serializable.
Unions are unsupported.

Derived values encode fields in declaration order. Enum variants use their Rust
identifier spelling as a string tag, rather than a numeric discriminant.
Changing a field's position or encoding, or renaming a variant, changes the wire
format. Field names and Rust type names are not encoded.

Missing trailing fields use `Default::default()` unless a field selects another
policy:

```rust
use derse::{Deserialize, Serialize};

fn default_port() -> u16 { 8080 }

#[derive(Serialize, Deserialize)]
struct Config {
    #[derse(required)]
    host: String,
    #[derse(default = "default_port")]
    port: u16,
    retries: u32,
}
```

| Field attribute | Behavior when the enclosing body is empty |
| --- | --- |
| Omitted, or `#[derse(default)]` | Use the field type's `Default` implementation. |
| `#[derse(required)]` | Call the field decoder even on empty input. |
| `#[derse(default = "path::function")]` | Call a function taking no arguments and returning the field type. |

Required fields and custom defaults remove the `Default` bound. A required
zero-byte type can still decode from empty input. Defaults never recover a
partially present or invalid field. These attributes affect deserialization
only; they do not omit serialized fields.

Appending fields with suitable defaults lets new readers accept old messages.
Old readers skip unrecognized bytes at the end of a derived body. New enum
variants remain errors for readers that do not know their tags. Compatibility
still depends on preserving the meaning and encoding of existing fields.

## Types and features

Built-in implementations cover primitive values, strings and byte slices,
tuples, arrays, common collections, paths, C strings, socket addresses, and
durations. See the
[wire format](https://github.com/SF-Zhou/derse/blob/main/docs/wire-format.md)
for their exact layouts and current limitations.

Arrays `[T; N]` support lengths `0..=32` and have no length prefix. Deserialization
constructs the array directly. Serializing a longer array fails during code
generation (`cargo build`); `cargo check` alone does not evaluate this assertion.
Byte slices and `Vec<u8>` have a length prefix, unlike fixed-size byte arrays.

| Feature | Additional implementation |
| --- | --- |
| `compact_str` | `compact_str::CompactString` |
| `tinyvec` | `tinyvec::TinyVec` |
| `full` | Both optional integrations |

No optional integrations are enabled by default. Enable a feature through the
dependency's `features` list, for example `features = ["full"]`.

## Documentation and development

- [API documentation](https://docs.rs/derse) (latest published version)
- [Wire format](https://github.com/SF-Zhou/derse/blob/main/docs/wire-format.md)
- [Development and testing](https://github.com/SF-Zhou/derse/blob/main/CONTRIBUTING.md)
- [Release guide](https://github.com/SF-Zhou/derse/blob/main/docs/releasing.md)
- [Changelog](https://github.com/SF-Zhou/derse/blob/main/CHANGELOG.md)

Run `cargo test --workspace --all-features` for the workspace tests and documentation
examples. CI requires 100% Rust line coverage across the runtime and derive crates.

## License

Licensed under either the [MIT license](https://github.com/SF-Zhou/derse/blob/main/LICENSE-MIT)
or the [Apache License, Version 2.0](https://github.com/SF-Zhou/derse/blob/main/LICENSE-APACHE),
at your option.
