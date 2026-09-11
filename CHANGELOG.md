# Changelog

Notable changes to `derse` and `derse-derive` are recorded here, with the newest
changes first. Tracking starts at `derse` 0.1.34 and `derse-derive` 0.1.15;
earlier releases are not backfilled. Starting with the planned `0.2.0-alpha`,
both crates share one release version. `Unreleased` entries describe the current
checkout and do not indicate that a version has been published.

## Unreleased

### Added

- A workspace version tool and CI checks for synchronized package versions,
  exact internal dependencies, and the non-published test fixture.
- A wire-format reference, development guide, and workspace release procedure,
  including prerelease selection and recovery from a partially completed release.
- Field attributes for derived deserialization: `#[derse(required)]`,
  `#[derse(default)]`, and `#[derse(default = "path::to_function")]`. Required
  fields and custom defaults do not require the field type to implement `Default`.
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
  Arrays are constructed directly for lengths `0..=32`.
- Reject serialization of arrays longer than 32 elements at compile time,
  keeping the supported lengths consistent with deserialization.
- Allow borrowed deserialized values to have lifetimes shorter than the input.
  Owned `Cow<T>` results no longer tie the outer `Cow` lifetime to the input.
- Validate derive inputs once and reuse the parsed field policies.
- Mark generated implementations with `#[automatically_derived]` so compiler
  tools recognize them as generated code.
- Require 100% line coverage for the workspace in CI.

### Fixed

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

- The version and documentation updates introduce no runtime or generated-code
  changes. Moving to `0.2.0-alpha` requires consumers to opt into that prerelease;
  existing `derse = "0.1"` requirements remain on the `0.1.x` series.
- Public serialization and deserialization trait method signatures are unchanged.
  Existing calls and manual trait implementations keep the same signatures.
- Field order, length prefixes, enum name tags, and array/tuple layouts are
  unchanged. Fixing calls to same-named inherent methods can change the emitted
  bytes: a field whose inherent method writes `99` but whose `Serialize`
  implementation writes its value `7` now encodes that value as `7`.
- Fields accepted by the old derive solely through an inherent, dereferenced, or
  extension-trait `serialize_to` method now need to satisfy `derse::Serialize`.
- Serializing arrays longer than 32 elements now fails during code generation
  (`cargo build`). This also applies to byte arrays and generic wrappers;
  `cargo check` alone does not evaluate the length assertion.
- Missing trailing fields still use `Default` unless a new field attribute selects
  another policy. Partially present fields continue to report decoding errors.
- Correcting the `actual` field in `Error::DataIsShort` changes the diagnostic
  value and, when that error value is serialized, its bytes; the error type's
  layout is unchanged.

### Known issues

- The documentation review identified existing `DownwardBytes` storage safety
  and overflowing `Duration` input issues. These still need separate fixes; see
  [development notes](CONTRIBUTING.md#existing-implementation-issues).

## 0.1.34

Starting baseline for this changelog: `derse` 0.1.34 and `derse-derive` 0.1.15.
