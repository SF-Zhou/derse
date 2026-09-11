# Releasing the workspace

## One version for both crates

`derse` and `derse-derive` are one release unit. Every workspace member inherits
`workspace.package.version`, including the non-published `derse-renamed-tests`
fixture. Publish both public crates at each release, even when one crate's source
has not changed. Use one Git tag, `v<VERSION>`, for the pair.

The runtime's dependency on `derse-derive` is an exact version requirement in
`workspace.dependencies`. Generated code must run against the runtime version
it was tested with. Cargo does not substitute `workspace.package.version` into
dependency requirements, so update both entries together when releasing.
This is an intended use of
[exact proc-macro dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#version-requirement-syntax).

The derive crate's development dependency on `derse` stays path-only. Adding a
registry version would make packaging the macro require the not-yet-published
runtime. The renamed test fixture also uses a path-only dependency and must keep
`publish = false`.

## Prerelease progression

The first unified release is planned as `0.2.0-alpha`; after testing, release
`0.2.0`. Both names are valid SemVer versions. If another alpha is needed, use
`0.2.0-alpha.1`, then `0.2.0-alpha.2`, and so on. A published version cannot be
overwritten. Stable `0.2.0` is a new publication of both crates, not a rename of
the alpha.

Cargo's [prerelease selection rules](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#pre-releases)
matter to consumers:

| Dependency requirement | Selection |
| --- | --- |
| `derse = "0.1"` | Stays on the `0.1.x` series. |
| `derse = "0.2"` | Selects stable `0.2.x` releases; excludes alpha versions. |
| `derse = "0.2.0-alpha"` | Can upgrade to later prereleases of `0.2.0` and compatible stable releases. |
| `derse = "=0.2.0-alpha"` | Stays on that exact alpha until the requirement changes. |

The README uses an exact version so alpha testing is reproducible. After the
stable release, applications may use `derse = "0.2"` for compatible updates.
The runtime always pins its own derive dependency exactly, including on stable
releases. Because this is still a `0.x` library, moving from `0.1` to `0.2` is the
appropriate Cargo version boundary for the source compatibility changes in the
[changelog](../CHANGELOG.md). This does not itself change the wire format.

## Prepare and verify

Use a Unix host and Cargo 1.90 or newer. Cargo 1.90 introduced
[workspace publishing](https://blog.rust-lang.org/2025/09/18/Rust-1.90.0/), which
packages and verifies interdependent new versions together.

Update `workspace.package.version` and the exact `derse-derive` requirement in
the root `Cargo.toml`, along with the installation version in `README.md`. For
example, promoting the alpha to stable sets the package version to `0.2.0` and
the internal dependency requirement to `=0.2.0`.

Then run these commands from the workspace root:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings -D missing_docs" cargo doc --workspace --all-features --no-deps
cargo llvm-cov --workspace --release --all-features --fail-under-lines 100
```

Also run the [buffer Miri checks](../CONTRIBUTING.md#buffer-memory-safety) and
[big-endian IPv4 checks](../CONTRIBUTING.md#big-endian-ipv4-decoding). CI runs these
in its dedicated `miri` job alongside the stable build and coverage checks.

Review the changes and prepare the changelog's release section. Keep `Unreleased`
for later work; only record a publication date after the release actually occurs.
Check README wording when changing from a prerelease to stable.

Commit the reviewed release state before the final packaging rehearsal:

```sh
cargo publish --workspace --all-features --registry crates-io --dry-run
```

Use `--allow-dirty` only for an earlier local rehearsal of uncommitted changes.
Inspect the `derse-<VERSION>.crate` and `derse-derive-<VERSION>.crate` archives
under `target/package/` (workspace publishing stages them in `tmp-registry/`).
Both must contain the correct normalized manifest, README, and both license
files. The crate directories link to the root license texts; Cargo includes
their contents in the archives.

The dry run uploads nothing. Cargo stages the new macro package locally, verifies
the runtime against it, and skips the non-published test fixture. Use a direct
crates.io source for this step: a global `source.crates-io.replace-with` mirror
can bypass the staged package and fail to find the unpublished macro version.
`--registry crates-io` selects the upload registry but does not disable source
replacement in Cargo configuration.

## Publish from a tag

The [release workflow](../.github/workflows/release.yml) publishes both crates when
a `v*` tag is pushed. Before the first automated release, configure
[crates.io trusted publishing](https://crates.io/docs/trusted-publishing) for both
`derse` and `derse-derive` with these values:

| Setting | Value |
| --- | --- |
| Repository owner | `SF-Zhou` |
| Repository name | `derse` |
| Workflow filename | `release.yml` |
| Environment | `release` |

Configure the GitHub `release` environment to allow the release tags. The workflow
uses a temporary crates.io token from `rust-lang/crates-io-auth-action`; a stored
registry token secret is unnecessary.

Once verification is complete and the release has been authorized, tag the
reviewed release commit as `v<workspace.package.version>` and push the tag. For
the planned alpha:

```sh
git tag v0.2.0-alpha
git push origin v0.2.0-alpha
```

The workflow runs `cargo publish --workspace --all-features --registry crates-io`
using stable Rust and a temporary registry token.
Complete the preparation checks above before pushing the tag; the release job
does not rerun the full test and Miri suites.

Cargo publishes `derse-derive` before `derse`. Workspace publication is not an
atomic registry operation: an interruption may leave only one crate published.
Check which versions reached crates.io and, without changing their contents,
authenticate locally through Cargo and retry only the missing package, for
example `cargo publish -p derse --all-features --registry crates-io`. Do not bump
just one crate to recover a partial release.

After both versions are available, record the release date and publish release
notes for the existing tag. Keep later changes under `Unreleased`. See the
[Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
for registry behavior and the permanence of published versions.
