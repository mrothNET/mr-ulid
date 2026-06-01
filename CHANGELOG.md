# Changelog

## 3.0.1

### Improvements

- Generalized the length error messages from "string is too short/long" to "input is too short/long", since they also apply to byte-slice inputs.
- Added a minimum supported Rust version (MSRV) of 1.85 to `Cargo.toml`.
- Expanded CI: OS × Rust-version matrix, `cargo fmt --check`, Clippy, and an MSRV job.

## 3.0.0

### Breaking Changes

- Renamed `Error::ToShort` to `Error::TooShort` and `Error::ToLong` to `Error::TooLong`.
- Updated `rand` dependency from 0.9 to 0.10.

### Bug Fixes

- Fixed non-working `TryFrom<&[u8]>` for both `Ulid` and `ZeroableUlid`.
- Fixed typos in doc comments across.
- Fixed compiler warnings for lifetime elisions.

### Improvements

- Expanded test suite, covering all `From`/`TryFrom` implementations, constants, `Default`, `try_new`, known-value decomposition, parse/display round-trips, byte conversions, cross-type conversions, datetime, ordering, hashing, `Clone`/`Copy`, `Error` type traits, serde serialization, and unsafe constructors.
- Overhauled README.
- Added CHANGELOG.md.

## 2.0.0

- Renamed `Ulid::generate()` to `Ulid::new()`.

## 1.2.0

- Switched to Rust Edition 2024.

## 1.1.2

- Updated dependency: `rand` 0.9.

## 1.1.1

- Fixed typos and wording.
- Simplified `from_parts()`.

## 1.1.0

- Implemented `Debug` trait for `EntropySourceHandle`.
- Fixed debug format for `ZeroableUlid`.

## 1.0.1

- Simplified ULID generation.

## 1.0.0

- Initial public release.
