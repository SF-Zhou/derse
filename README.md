# derse

[![Rust](https://github.com/SF-Zhou/derse/actions/workflows/rust.yml/badge.svg)](https://github.com/SF-Zhou/derse/actions/workflows/rust.yml)
[![Crates.io Version](https://img.shields.io/crates/v/derse)](https://crates.io/crates/derse)
[![codecov](https://codecov.io/gh/SF-Zhou/derse/graph/badge.svg?token=8I6CQT5VJ5)](https://codecov.io/gh/SF-Zhou/derse)
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FSF-Zhou%2Fderse.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2FSF-Zhou%2Fderse?ref=badge_shield)

A simple binary serialization protocol for Rust.

## Usage

To use this library, add the following to your Cargo.toml:

```toml
[dependencies]
derse = "0.1"
```

Then, you can import and use the components as follows:

```rust
use derse::{Deserialize, DownwardBytes, Serialize};

// 1. serialization for basic types.
let ser = "hello world!";
let bytes = ser.serialize::<DownwardBytes>().unwrap();
let der = String::deserialize(&bytes[..]).unwrap();
assert_eq!(ser, der);

// 2. serialization for custom structs.
#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct Demo {
    a: i32,
    b: String,
    c: Vec<String>,
}
let ser = Demo::default();
let bytes = ser.serialize::<DownwardBytes>().unwrap();
let der = Demo::deserialize(&bytes[..]).unwrap();
assert_eq!(ser, der);
```


Fixed-size arrays `[T; N]` support serialization and deserialization for lengths
`0..=32`. Serializing a longer array fails at compile time during code generation
(`cargo build`); `cargo check` alone does not evaluate this assertion.
Deserialization constructs the array directly, without an intermediate buffer.
Their encoding contains the elements in order, without a length prefix.

## Derived types and compatibility

The derive macros support named, tuple, and unit structs and enum variants, as
well as generic fields and borrowed data. Trait bounds are inferred from the
fields; marker types such as `PhantomData<T>` do not require `T` to be serializable.

Each derived value has a byte-length prefix. Fields are encoded in declaration
order, and enum variants are identified by name. Keep existing field order and
encodings unchanged when evolving a type: new readers fill in missing trailing
fields, and old readers skip unknown trailing fields. Renaming an enum variant
changes its encoded tag.

By default, every missing trailing field uses `Default::default()`. Field
attributes can require decoding or supply a different fallback:

```rust
use derse::{Deserialize, Serialize};

fn default_port() -> u16 {
    8080
}

#[derive(Serialize, Deserialize)]
struct Config {
    #[derse(required)]
    host: String,
    #[derse(default = "default_port")]
    port: u16,
    retries: u32,
}
```

`required` always calls the field's deserializer, which reports missing or
invalid data according to that type's encoding. A custom default function takes
no arguments and returns the field type. Both options remove the field's
`Default` requirement. Defaults apply only when the enclosing value's remaining
body is empty; a partially present field still reports a decoding error.

## Tests and coverage

Run all workspace tests and the same coverage check used by CI:

```sh
cargo test --workspace --all-features
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --release --features full --fail-under-lines 100
```

CI requires 100% line coverage across the runtime and derive crates.

## License
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FSF-Zhou%2Fderse.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2FSF-Zhou%2Fderse?ref=badge_large)
