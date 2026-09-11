# Development

## Workspace layout

| Path | Responsibility |
| --- | --- |
| `derse/src` | Public traits, byte readers/writer, errors, and built-in type implementations. |
| `derse-derive/src` | Input validation, generated implementations, and macro unit tests. |
| `derse/tests` | Round trips, compatibility regressions, and compiler diagnostics. |
| `tests/renamed` | A non-published crate that imports the runtime as `ds`. |

Use a Unix host and the current stable Rust toolchain. The repository does not
currently declare or test a minimum supported Rust version. Release commands
using workspace publishing need Cargo 1.90 or newer; this tooling requirement is
separate from the library's compiler requirements.

## Local checks

Run from the workspace root:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --all-features --no-deps
```

`cargo test` includes executable Rust documentation examples. Use
`cargo doc -p derse --all-features --no-deps --open` to browse the API locally.
When changing an optional integration, also test the affected feature by itself
and check the default-feature configuration as appropriate.

Run complete tests from the repository checkout. The derive crate has a path-only
development dependency on the runtime so its tests can resolve the runtime name.
Cargo removes that dependency from the published archive, avoiding a release
dependency cycle. Running the macro unit tests directly from that archive does
not recreate the workspace test environment.

## Big-endian IPv4 decoding

CI runs the IPv4 decoding tests on a big-endian target in its dedicated Miri job.
To run the same check locally, install nightly Rust with the `miri` and `rust-src`
components:

```sh
rustup toolchain install nightly --profile minimal --component miri --component rust-src
MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test -p derse --lib --all-features --target s390x-unknown-linux-gnu ipv4_decode_
```

These tests decode fixed input bytes. They check IP decoding without exercising
`DownwardBytes` or validating the rest of the runtime on that target.

## Coverage

Install the LLVM component and coverage tool once:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked
```

Run the same Rust line-coverage threshold as CI:

```sh
cargo llvm-cov --workspace --release --all-features --fail-under-lines 100 --lcov --output-path target/coverage.lcov
```

For local inspection, generate HTML from the collected data:

```sh
cargo llvm-cov report --release --html --output-dir target/coverage-html
```

The target is 100% line coverage across both Rust crates. This does not claim
100% branch or region coverage. Macro expansion tests exercise the generator;
integration tests exercise the generated code, including malformed data and
renamed dependencies. Generated implementations carry `#[automatically_derived]`
so compiler tooling recognizes them. Do not hide executable source from the
coverage report to meet the threshold.

## Compiler tests and generated files

`derse/tests/build` contains trybuild fixtures and checked-in `.stderr` snapshots.
When a diagnostic intentionally changes, regenerate and review its snapshot:

```sh
TRYBUILD=overwrite cargo test -p derse --test test_derive --all-features
```

Array serialization limits require code generation: `cargo check` cannot detect
the inline const assertion for lengths greater than 32. The
`test_array_serialize_limit` integration test runs isolated `cargo build` probes
for method calls, UFCS, derives, and generic wrappers, as well as valid boundary
cases. Its temporary crates and build directories are cleaned up automatically.

Commit the fixtures and `.stderr` files. Build output, LLVM profiles, coverage
reports, and `derse/wip` are generated artifacts covered by `.gitignore`.
`Cargo.lock` is currently untracked for this library workspace; a release checkout
should keep its generated lockfile through verification and publication so the
same resolved dependencies are checked and packaged.

## Documentation and compatibility

Public rustdoc describes behavior, input consumption, borrowing, and error
conditions. Internal comments should explain an invariant or implementation
choice. Keep code examples executable where practical. Update the
[wire format](docs/wire-format.md) and [changelog](CHANGELOG.md) when behavior
changes, and follow the [release guide](docs/releasing.md) for version changes.

A matching Rust signature does not prove wire compatibility. Review exact bytes,
field order, length prefixes, missing-field behavior, and malformed-input errors.
For collections, distinguish iteration order from sorted order. When changing
serialization code, also check partial writes and reads, and cleanup after an
element decoder fails or panics.

## Buffer memory safety

`DownwardBytes` owns a raw byte pointer, capacity, and encoded length. Its size
remains three machine words. The unused prefix needs no initialization or zeroing;
only the tail `capacity - length..capacity` contains encoded bytes.

Keep these invariants when changing its storage or operations:

- The pointer owns a `Vec<u8>`-compatible allocation, or is non-null and dangling
  at zero capacity. The encoded length never exceeds capacity, and capacity never
  exceeds `isize::MAX`.
- Allocation and cleanup transfer ownership through temporary `Vec<u8>` values
  whose length is always zero. They use the original allocation pointer and
  actual capacity. No `set_len` call or initialized view of unused storage is
  needed, and exactly one owner releases each allocation.
- Prepending copies into unused space before publishing the new length. Shared
  slices expose only the initialized tail; all writes require exclusive access.
  Clearing changes only the encoded length.
- Growth allocates a replacement, copies the existing initialized tail to its
  new end, and then releases the old allocation. Retain the current capacity
  policy, public interfaces, and wire order.

CI runs the focused buffer tests under Miri with all features and strict
provenance checking. With nightly Rust and the `miri` and `rust-src` components
installed, run the same check locally:

```sh
MIRIFLAGS="-Zmiri-strict-provenance" cargo +nightly miri test -p derse --test test_downward_bytes --all-features
```

Other current format limits, including unchecked integer narrowing and varint
overflow bits, are described in the wire-format reference. The coverage threshold
does not replace reviewing these invariants or testing additional targets.
