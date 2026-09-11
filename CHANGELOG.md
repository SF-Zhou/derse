# Changelog

Notable changes to `derse` and `derse-derive` are recorded here, with the newest
changes first. Tracking starts at `derse` 0.1.34 and `derse-derive` 0.1.15;
earlier releases are not backfilled. Starting with `0.2.0-alpha`,
both crates share one release version. `Unreleased` entries describe the current
checkout and do not indicate that a version has been published.

## Unreleased

## 0.2.0

### Changed

- Promote `derse` and `derse-derive` from `0.2.0-alpha` to the stable `0.2.0`
  release, keeping the runtime's derive dependency pinned to the exact matching
  version. Update installation examples to use `derse = "0.2"`.
- Runtime and derive source code, public APIs, and wire formats are unchanged
  from `0.2.0-alpha`. The alpha release's compatibility notes below still apply
  when upgrading from `0.1.x`.

## 0.2.0-alpha - 2026-09-11

### Added

- A dedicated Miri CI job for buffer memory safety, IPv4 decoding on a big-endian
  target, and input-length overflow on a 32-bit target, with all features and
  strict provenance checking enabled.
- A tag-triggered release workflow using crates.io trusted publishing to publish
  both crates in dependency order.
- A wire-format reference, development guide, and workspace release procedure,
  including prerelease selection and recovery from a partially completed release.
- Field attributes for derived deserialization: `#[derse(required)]`,
  `#[derse(default)]`, and `#[derse(default = "path::to_function")]`. Required
  fields and custom defaults do not require the field type to implement `Default`.
- A `#[derse(recursive)]` field marker for generic recursion hidden behind type
  aliases or qualified paths. It skips inferred serialization and deserialization
  bounds for the whole field while preserving the selected missing-field policy.
- Automatic field trait bounds for derived implementations, including generic
  fields, associated types, const generics, and recursive types.
- Deserialization of zero-length arrays, extending the supported array lengths
  from `1..=32` to `0..=32`.
- Regression tests for derive expansion, renamed dependencies, fragmented input,
  malformed data, and array element cleanup after errors or panics.

### Changed

- Unify all workspace package versions at `0.2.0-alpha`. Publish `derse` and
  `derse-derive` together, with the runtime depending on the exact matching macro
  version instead of the open-ended `>=0.1.14` requirement.
- Rewrite API documentation and implementation comments to describe encoding
  order, borrowing, input consumption, missing-field policies, and current limits.
  Add executable documentation examples. Include the shared README and both
  license texts in each published crate.
- Check public API documentation and rehearse workspace publication in CI.
- Simplify collection and tuple macros and generate array deserializers from a
  single length list, preserving encoding order and element trait requirements.
  Derive the serialization limit from the same list. Arrays are constructed
  directly for lengths `0..=32`.
- Reject serialization of arrays longer than 32 elements at compile time,
  keeping the supported lengths consistent with deserialization.
- Allow borrowed deserialized values to have lifetimes shorter than the input.
  Owned `Cow<T>` results no longer tie the outer `Cow` lifetime to the input.
- Validate derive inputs once and reuse the parsed field policies.
- Mark generated implementations with `#[automatically_derived]` so compiler
  tools recognize them as generated code.
- Require 100% line coverage for the workspace in CI.

### Fixed

- Reject a combined `BytesArray` fragment length above `usize::MAX` with a
  consistent constructor panic, instead of silently wrapping in release builds.
- Return `Error::InvalidValue("tinyvec capacity overflow")` for a `TinyVec` count
  whose element allocation exceeds Rust's layout limit, before allocating or
  reading elements. Preserve its inline storage and preallocation policy.
- Infer complete generic field bounds even when a type parameter already has an
  explicit trait bound, preserving collection and wrapper requirements such as
  `Eq` and `Hash`. Keep the generated input lifetime independent of existing bounds.
- Stop identifying existing traits by their names, so unrelated `Serialize`,
  `Deserialize`, and `Default` traits cannot suppress required field bounds.
- Treat qualified paths to another type as complete field types, including paths
  whose final name matches the type being derived.
- Dereference decoded enum tags directly so caller-defined `as_ref` methods do
  not make generated deserializers ambiguous.
- Generate valid runtime paths for keyword dependency aliases such as `async`.
- Give `DownwardBytes` separate pointer, capacity, and initialized-tail length
  fields instead of using `Vec::set_len` over uninitialized bytes. Transfer
  allocation ownership through zero-length `Vec<u8>` values for allocation and
  cleanup, without zeroing unused storage. Preserve the three-word buffer size,
  public interfaces, encoded bytes, and capacity growth policy.
- Return `Error::InvalidValue` with the message `duration overflow` when decoding
  a `Duration` whose nanosecond carry overflows the seconds count, instead of
  panicking. Preserve nanosecond normalization, the 12-byte encoding, public
  interfaces, and the existing read order and short-input behavior.
- Decode IPv4 addresses correctly on big-endian hosts, including IPv4 socket
  addresses. Preserve serialized bytes, the single four-byte read, and existing
  short-input errors and cursor behavior.
- Use explicit trait calls in generated code so same-named inherent methods
  cannot override serialization or detailed deserialization.
- Resolve renamed `derse` dependencies consistently and prevent generated names
  from colliding with caller identifiers, lifetimes, or custom default paths.
- Preserve explicit generic bounds and handle independent borrowed lifetimes
  without introducing ambiguous or cyclic inferred constraints.
- Decode enum names that span multiple input fragments.
- Handle zero-length `BytesArray::advance` and `BytesArray::pop` operations on
  empty input successfully, without consuming input.
- Report the actual remaining length in `BytesArray::advance` short-input errors.
- Report unsupported unions and invalid derive attributes as compilation errors
  with source locations.

### Compatibility notes

- Moving to `0.2.0-alpha` requires consumers to opt into that prerelease;
  existing `derse = "0.1"` requirements remain on the `0.1.x` series.
- Public serialization and deserialization trait method signatures are unchanged.
  Existing calls and manual trait implementations keep the same signatures.
- Field order, length prefixes, enum name tags, and array/tuple layouts are
  unchanged. Fixing calls to same-named inherent methods can change the emitted
  bytes: a field whose inherent method writes `99` but whose `Serialize`
  implementation writes its value `7` now encodes that value as `7`.
- Fields accepted by the old derive solely through an inherent, dereferenced, or
  extension-trait `serialize_to` method now need to satisfy `derse::Serialize`.
- Generic recursive aliases that relied on explicit parameter bounds to suppress
  field inference, and recursion written through qualified module paths, may now
  need `#[derse(recursive)]` on the field. Supply any required generic bounds
  explicitly, including for nonrecursive parts of that field. The marker does
  not change encoded bytes or default behavior.
- Serializing arrays longer than 32 elements now fails during code generation
  (`cargo build`). This also applies to byte arrays and generic wrappers;
  `cargo check` alone does not evaluate the length assertion.
- Missing trailing fields still use `Default` unless a new field attribute selects
  another policy. Partially present fields continue to report decoding errors.
- Correcting the `actual` field in `Error::DataIsShort` changes the diagnostic
  value and, when that error value is serialized, its bytes; the error type's
  layout is unchanged.

## 0.1.34

Starting baseline for this changelog: `derse` 0.1.34 and `derse-derive` 0.1.15.
