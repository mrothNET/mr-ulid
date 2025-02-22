//! # Robust and Hassle-Free ULIDs
//!
//! This crate provides an implementation of ULIDs (Universally Unique Lexicographically Sortable Identifiers)
//! with an emphasis on correctness, resilience, and hassle-free usability, in that order.
//! It enables the generation and manipulation of ULIDs that are guaranteed to be unique
//! and strictly monotonically increasing in any circumstances.
//!
//! ## Generating ULIDs
//!
//! ULIDs are generated using the [`Ulid::generate()`] method:
//!
//! ```
//! use mr_ulid::Ulid;
//!
//! let u = Ulid::generate();
//! ```
//!
//! Each ULID generated is guaranteed to be unique and strictly monotonically increasing.
//! The generation is thread-safe, maintaining all guarantees even when ULIDs are produced
//! concurrently across multiple threads.
//!
//! ## Printing ULIDs and converting to Strings
//!
//! ULIDs implement the [`std::fmt::Display`] trait:
//!
//! ```
//! use mr_ulid::Ulid;
//!
//! let u = Ulid::generate();
//!
//! println!("Generated ULID: {u}");
//!
//! let s = u.to_string();
//!
//! ```
//!
//! ## Parsing ULIDs from Strings:
//!
//! ULIDs implements the [`std::str::FromStr`] trait and can be parsed with [`str::parse()`] method:
//!
//! ```
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! use mr_ulid::Ulid;
//!
//! // Method A
// cspell:disable-next-line
//! let u1: Ulid = "01JB5C84ZBM8QVBE5QRZW6HY89".parse()?;
//!
//! // Method B
// cspell:disable-next-line
//! let u2 = "01JB5C84ZBM8QVBE5QRZW6HY89".parse::<Ulid>()?;
//! # Ok(()) }
//! ```
//!
//! ## Serializing and Deserializing using `Serde` (JSON)
//!
//! For serializing/deserializing the feature flag `serde` needs to be enabled:
//!
//! ```bash
//! cargo add mr-ulid -F serde
//! ```
//!
//! Once the `serde` feature is enabled, ULIDs implement the `Serialize` and `Deserialize` traits:
//!
//! ```
//! # use std::error::Error;
//! # fn main() -> Result<(), Box<dyn Error>> {
//! # #[cfg(feature = "serde")]
//! # {
//! use mr_ulid::Ulid;
//! # use serde_derive as serde;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! struct Example {
//!     id: Ulid,
//!     data: String,
//! }
//!
//! let e1 = Example {
//!     id: Ulid::generate(),
//!     data: "Hello, World!".to_string(),
//! };
//!
//! let s = serde_json::to_string(&e1)?;
//!
//! println!("JSON: {s}");
//!
//! let e2: Example = serde_json::from_str(&s)?;
//!
//! assert_eq!(e1, e2);
//! # }
//! # Ok(()) }
//! ```
//!
//! ## Guarantees
//!
//! A notable feature of this crate is the guarantee that a sufficient number
//! of ULIDs can be generated at any time without the random part overflowing
//! and the guarantees of uniqueness and strict monotonicity is maintained
//! under all circumstances.
//!
//! The 80-bit random component of a ULID is slightly reduced by 10<sup>10</sup> values,
//! resulting in a negligible reduction in entropy of approximately 0.000000000001%.
//! This ensures that at least 10<sup>10</sup> ULIDs can be generated per _millisecond_,
//! equating to 10<sup>13</sup> ULIDs per _second_.
//! Such capacity exceeds the capabilities of current systems by magnitudes.
//!
//! In the very unlikely event that a system could generate more than 10<sup>13</sup> ULIDs
//! per second, any overflowing random part is projected into the next millisecond.
//! There, the full range of 2<sup>80</sup> (ca. 10<sup>24</sup>) is available.
//!
//! ## ULID Types
//!
//! - [`Ulid`]: This is the preferred type for most use cases and represents a ULID that can never be zero.
//! - [`ZeroableUlid`]: This alternative type allows for zero values ULIDs (e.g., `"00000000000000000000000000"`).
//!
//! In idiomatic Rust code, if an absent ULID is needed, it is best represented as [`Option<Ulid>`](Ulid).
//! However, for use cases that may represent absent ULIDs with a zero ULID,
//! the [`ZeroableUlid`] may be an easier choice.
//!
//! ## Feature Flags
//!
//! - **`rand`**: Utilizes the `rand` crate as the source for random numbers, enabled by default.
//! - **`serde`**: Provides support for serialization and deserialization via `Serde`, optional.
//!

mod base32;
mod error;
mod generator;
mod nonzero;
#[cfg(feature = "serde")]
mod serde;
mod util;
mod zeroable;

use std::borrow::Cow;

pub use error::Error;
#[cfg(feature = "rand")]
pub use generator::STANDARD_ENTROPY_SOURCE;
pub use generator::{EntropySource, EntropySourceHandle, NO_ENTROPY_SOURCE, set_entropy_source};
pub use nonzero::Ulid;
pub use zeroable::ZeroableUlid;

const RESERVED: u128 = 10_000_000_000;

const RANDOM_BITS: u32 = 80;
const RANDOM_MASK: u128 = (1 << RANDOM_BITS) - 1;
const RANDOM_GEN_MAX: u128 = RANDOM_MASK - RESERVED;

const TIMESTAMP_BITS: u32 = 48;
const TIMESTAMP_MAX: u64 = (1 << TIMESTAMP_BITS) - 1;
const TIMESTAMP_MASK: u128 = ((1 << TIMESTAMP_BITS) - 1) << RANDOM_BITS;

/// Canonicalizes a ULID string by converting it to a standard format.
///
/// Takes a ULID string and returns the canonicalized version:
/// Letters 'i', 'l', and 'o' are replaced by their corresponding digits '1' and `0`,
/// and all characters are converted into uppercase.
///
/// If the input is already in canonical form, it returns a borrowed version of the input string
/// without allocating a new `String`.
///
/// # Errors
///
/// The string must be a valid ULID. It must have the correct length (26) and contain only valid characters,
/// and it is not to be overflowed. If not, an error is returned.
///
/// # Example
///
/// ```
// cspell:disable-next-line
/// let s = "olixjazthsfjzt7wd6j8ir92vn";
///
// cspell:disable-next-line
/// assert_eq!(mr_ulid::canonicalize(s), Ok("011XJAZTHSFJZT7WD6J81R92VN".into()));
/// ```
///
pub fn canonicalize(ulid: &str) -> Result<Cow<str>, Error> {
    let mut buffer = *util::as_array(ulid.as_bytes())?;
    let cleaned = base32::canonicalize(&mut buffer)?;

    if cleaned == ulid {
        Ok(ulid.into())
    } else {
        Ok(cleaned.to_string().into())
    }
}
/// Checks a ULID string for validity.
///
/// To be valid, a ULID must have the correct length (26) and contain only valid characters,
/// and not overflowed.
///
/// It is not checked, if the ULID has a zero value or if the ULID is in its canonical form.
///
/// # Errors
///
/// If the ULID string is not valid, an appropriate error is returned.
///
/// # Example
///
/// ```
// cspell:disable-next-line
/// assert!(mr_ulid::validate("olixjazthsfjzt7wd6j8ir92vn").is_ok());
// cspell:disable-next-line
/// assert!(mr_ulid::validate("011XJAZTHSFJZT7WD6J81R92VN").is_ok());
///
/// assert!(mr_ulid::validate("00000000000000000000000000").is_ok());
/// assert!(mr_ulid::validate("7FFFFFFFFFFFFFFFFFFFFFFFFF").is_ok());
/// assert_eq!(mr_ulid::validate("80000000000000000000000000"), Err(mr_ulid::Error::InvalidChar));
///
/// assert_eq!(mr_ulid::validate("0000000000000000000000u89$"), Err(mr_ulid::Error::InvalidChar));
/// assert_eq!(mr_ulid::validate("xxxxxxxxxxxxxxxxxxxxxx"), Err(mr_ulid::Error::ToShort));
/// assert_eq!(mr_ulid::validate("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"), Err(mr_ulid::Error::ToLong));
/// ```
pub fn validate(ulid: &str) -> Result<(), Error> {
    let buffer = util::as_array(ulid.as_bytes())?;
    base32::validate(buffer)
}

#[cfg(test)]
mod tests;
