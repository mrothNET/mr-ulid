# mr-ulid

[![Crates.io](https://img.shields.io/crates/v/mr-ulid)](https://crates.io/crates/mr-ulid)
[![Documentation](https://img.shields.io/docsrs/mr-ulid)](https://docs.rs/mr-ulid)
[![Dependencies](https://deps.rs/repo/github/mrothNET/mr-ulid/status.svg)](https://deps.rs/repo/github/mrothNET/mr-ulid)
[![License](https://img.shields.io/crates/l/mr-ulid)](https://github.com/mrothNET/mr-ulid/blob/main/LICENSE)

A Rust implementation of [ULIDs](https://github.com/ulid/spec) (Universally Unique Lexicographically Sortable Identifiers) with a focus on correctness and ease of use.

Generated ULIDs are guaranteed to be unique and strictly monotonically increasing, even across threads. The random component is overflow-proof by design -- see [Overflow Protection](#overflow-protection) for details.

## Usage

```toml
[dependencies]
mr-ulid = "3"
```

```rust
use mr_ulid::Ulid;

let u = Ulid::new();
println!("{u}");

let s = u.to_string();
let parsed: Ulid = s.parse().unwrap();
assert_eq!(u, parsed);
```

## Features

- **Overflow-proof generation** -- At least 10<sup>10</sup> ULIDs per millisecond without overflow or failure.
- **Strict monotonicity** -- Thread-safe generation; every ULID is greater than the previous one.
- **Non-zero type (`Ulid`)** -- Wraps `NonZero<u128>`, so `Option<Ulid>` is the same size as `Ulid` (16 bytes).
- **Zeroable type (`ZeroableUlid`)** -- For use cases that need a zero sentinel value.
- **Crockford Base32** -- Case-insensitive encoding with automatic `i`/`l` to `1` and `o` to `0` disambiguation.
- **Optional `serde` support** -- Enable the `serde` feature for string-based serialization.
- **Custom entropy sources** -- Swap in your own RNG via the `EntropySource` trait.
- **Minimal dependencies** -- Only `rand` (enabled by default). Disable with `default-features = false`.

## Serde

Enable the `serde` feature for JSON (and other format) support:

```toml
[dependencies]
mr-ulid = { version = "3", features = ["serde"] }
```

```rust
use mr_ulid::Ulid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Record {
    id: Ulid,
    data: String,
}
```

ULIDs are serialized as 26-character Crockford Base32 strings.

## Overflow Protection

The 80-bit random component is reduced by 10<sup>10</sup> values (a ~0.000000000001% reduction in entropy). This reserves enough space to guarantee at least 10<sup>10</sup> monotonically increasing ULIDs per millisecond -- equivalent to 10<sup>13</sup> per second -- without overflow or failure. This exceeds the capability of current hardware by orders of magnitude.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.

## License

[MIT](https://github.com/mrothNET/mr-ulid/blob/main/LICENSE)
