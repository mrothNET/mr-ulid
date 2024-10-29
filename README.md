# mr-ulid

<!--
[![Crates.io](https://img.shields.io/crates/v/mr-ulid)](https://crates.io/crates/mr-ulid)
[![Dependencies](https://deps.rs/repo/github/mrothNET/mr-ulid/status.svg)](https://deps.rs/repo/github/mrothNET/mr-ulid)
[![License](https://img.shields.io/crates/l/mr-ulid)](https://github.com/mrothNET/mr-ulid/blob/main/LICENSE)
[![Documentation](https://img.shields.io/docsrs/mr-ulid)](https://docs.rs/mr-ulid)
-->

**Robust and Hassle-Free ULIDs (Universally Unique Lexicographically Sortable Identifiers)**

`mr-ulid` is designed with a focus on correctness and ease of use. It ensures that ULIDs generated are unique and strictly monotonically increasing.
By providing both `Ulid` and `ZeroableUlid` types, it serves different application needs, whether you require non-zero guarantees or need to handle zero ULIDs.

## Key Features

- **Robust**: Generates ULIDs that are unique and strictly monotonically increasing under all circumstances, including threads, no failing, and no overflowing random part. See below for [Details](#Guarantees).
- **Hassle-Free**: Simple API for easy usage. Customize entropy source when needed.
- **Non-Zero ULIDs**: Provides both non-zero (`Ulid`) and zeroable (`ZeroableUlid`) types to suit different use cases.
- **Minimal Dependencies**: Actually no dependencies required, only `rand` enabled by default as Rust lacks a built-in random number generator.
- **Optional Features**: Supports `serde` for serialization and deserialization.

## Installation

Add `mr-ulid` to your `Cargo.toml`:

```toml
[dependencies]
mr-ulid = "1"
```

By default, the `rand` feature is enabled.

## Quickstart

```rust
use mr_ulid::Ulid;

fn main() {
    // Generate a ULID
    let u = Ulid::generate();

    // Print a ULID
    println!("Generated ULID: {u}");

    // Convert a ULID to a string
    let s = ulid.to_string();

    // Parse the string back into a ULID
    let parsed: Ulid = s.parse().unwrap();

    // Verify that the original and parsed ULIDs are the same
    assert_eq!(u, parsed);
}
```

### Serialization and Deserialization (JSON)

To enable serialization and deserialization, add `serde` and `serde_json` to your `Cargo.toml`, and enable the `serde` feature for `mr-ulid`:

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = { version = "1" }
mr-ulid = { version = "1", features = ["serde"] }
```

#### Example with `Serde`

```rust
use mr_ulid::Ulid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Example {
    id: Ulid,
    data: String,
}

fn main() {
    let example = Example {
        id: Ulid::generate(),
        data: "Hello, ULID!".to_string(),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&example).unwrap();
    println!("Serialized JSON: {json}");

    // Deserialize back to struct
    let deserialized: Example = serde_json::from_str(&json).unwrap();

    // Verify that the original and deserialized structs are the same
    assert_eq!(example, deserialized);
}
```

## Guarantees

A notable feature of this crate is the guarantee that a sufficient number of ULIDs can be generated at any time without failing or the random part overflowing.

The 80-bit random component of a ULID is slightly reduced by 10<sup>10</sup> values, resulting in a negligible reduction in entropy of approximately 0.000000000001%. This ensures that at least 10<sup>10</sup> ULIDs can be generated per _millisecond_, equating to 10<sup>13</sup> ULIDs per _second_. Such capacity exceeds the capabilities of current systems by magnitudes.

## Contributing

Contributions are welcome! Whether it's a bug fix, new feature, or improvement, your help is appreciated. Please feel free to open issues or submit pull requests on the [GitHub repository](https://github.com/mrothNET/mr-ulid).

## License

This project is licensed under the [MIT License](https://github.com/mrothNET/mr-ulid/blob/main/LICENSE).
