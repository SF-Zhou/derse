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


## License
[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2FSF-Zhou%2Fderse.svg?type=large)](https://app.fossa.com/projects/git%2Bgithub.com%2FSF-Zhou%2Fderse?ref=badge_large)