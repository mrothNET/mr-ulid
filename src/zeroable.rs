use std::{
    fmt,
    str::FromStr,
    time::{Duration, SystemTime},
};

use crate::{Error, RANDOM_BITS, RANDOM_MASK, Ulid, base32, generator, util};

/// A ULID with even the value zero allowed.
///
/// This flavour of ULID can be zero. Sometimes ULIDs with zero value are used to
/// signal the absence of a ULID. While this works, it could be considered a bad practice.
/// In Rust an [`Option<Ulid>`](Ulid) is a more idiomatic way to handle ULIDs with zero value.
/// However, if you need to parse zero value ULIDs, this type helps.
///
/// # Example
///
/// ```
/// use mr_ulid::ZeroableUlid;
///
/// let s = "00000000000000000000000000";
///
/// assert!(s.parse::<ZeroableUlid>().is_ok());
/// ```
#[allow(clippy::module_name_repetitions)]
#[derive(Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ZeroableUlid(u128);

impl ZeroableUlid {
    /// Minimum allowed [`ZeroableUlid`]
    ///
    /// The smallest value for [`ZeroableUlid`] is 0, because zero *is* explicitly allowed.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// assert_eq!(ZeroableUlid::MIN.to_u128(), 0);
    /// ```
    pub const MIN: Self = Self(0);

    /// Maximum allowed [`ZeroableUlid`]
    ///
    /// The largest value for [`ZeroableUlid`] is `u128::MAX`.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// assert_eq!(ZeroableUlid::MAX.to_u128(), u128::MAX);
    /// ```
    pub const MAX: Self = Self(u128::MAX);

    /// Creates a new `ZeroableUlid` with the value zero.
    ///
    /// This is normally not what you want, because each created `ZeroableUlid` has always value zero.
    ///
    /// Chances are high, you're looking for method [`ZeroableUlid::generate()`],
    /// which creates unique `ZeroableUlid`s which are never zero.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u1 = ZeroableUlid::new();
    /// let u2 = ZeroableUlid::new();
    ///
    /// assert!(u1 == u2); // They are not unique!
    ///
    /// assert!(u1.is_zero()); // They are both zero!
    /// assert!(u2.is_zero());
    ///
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Generates a new unique `ZeroableUlid`.
    ///
    /// The generated `ZeroableUlid`s are guaranteed to be unique and monotonically increasing and never zero.
    ///
    /// A lot of care is taken, that the `ZeroableUlid`s cannot overflow. You can create as many
    /// `ZeroableUlid`s as you like, and they will always be unique and strict monotonically increasing.
    ///
    /// # Panics
    ///
    /// With the standard entropy source ([`STANDARD_ENTROPY_SOURCE`][generator::STANDARD_ENTROPY_SOURCE]),
    /// this method will panic if the system date is somewhere after the year 10889 or before the Unix epoch (year 1970).
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u1 = ZeroableUlid::generate();
    /// let u2 = ZeroableUlid::generate();
    ///
    /// assert!(u1 != u2);
    /// assert!(u1 < u2);
    ///
    /// let t1 = u1.timestamp();
    /// let t2 = u2.timestamp();
    ///
    /// let r1 = u1.randomness();
    /// let r2 = u2.randomness();
    ///
    /// assert!((t1 < t2) || (t1 == t2 && r1 < r2));
    /// ```
    #[must_use]
    pub fn generate() -> Self {
        Self(generator::generate().unwrap())
    }

    /// Tests if a `ZeroableUlid` is zero.
    ///
    /// When a `ZeroableUlid` is zero, it returns `true`. Otherwise, it returns `false`.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u1 = ZeroableUlid::new();
    ///
    /// assert!(u1.is_zero());
    /// ```
    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    /// Returns the timestamp part of a `ZeroableUlid`.
    ///
    /// The timestamp is measured in milliseconds since the Unix epoch (1. January 1970).
    /// ULID timestamps are limited to 48 bits.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u = ZeroableUlid::generate();
    ///
    /// assert!(u.timestamp() > 1704067200000); // 1st January 2024
    /// ```
    #[must_use]
    pub const fn timestamp(self) -> u64 {
        (self.0 >> RANDOM_BITS) as u64
    }

    /// Returns the random part of a `ZeroableUlid`.
    ///
    /// The randomness of a `ZeroableUlid` is limited to 80 bits.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u = ZeroableUlid::generate();
    ///
    /// assert!(u.randomness() < (1<<80));
    /// ```
    ///
    #[must_use]
    pub const fn randomness(self) -> u128 {
        self.0 & RANDOM_MASK
    }

    /// Returns the timestamp part of a `ZeroableUlid` as a `SystemTime`.
    ///
    /// # Panics
    ///
    /// In Rust the allowed range for [`SystemTime`] is not defined.
    /// So this method may panic if the timestamp of the `ZeroableUlid` cannot represented with [`SystemTime`].
    /// On most common systems that will not happen.
    ///
    /// For a variant which never panics, see [`ZeroableUlid::try_datetime`].
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::SystemTime;
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u = ZeroableUlid::generate();
    ///
    /// assert!(u.datetime() <= SystemTime::now());
    /// ```
    #[must_use]
    pub fn datetime(self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_millis(self.timestamp())
    }

    /// Converts this `ZeroableUlid` to a [`Ulid`].
    ///
    /// When the `ZeroableUlid` is zero, `None` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u1 = ZeroableUlid::generate();
    ///
    /// assert!(!u1.is_zero());
    /// assert!(u1.to_ulid().is_some());
    ///
    /// let u2 = ZeroableUlid::new(); // Creates a ZeroableUlid with value zero
    ///
    /// assert!(u2.is_zero());
    /// assert!(u2.to_ulid().is_none());
    ///
    /// ```
    #[must_use]
    pub const fn to_ulid(self) -> Option<Ulid> {
        Ulid::from_u128(self.0)
    }

    /// Creates a `ZeroableUlid` from a [`Ulid`].
    ///
    /// This method always succeeds, as every [`Ulid`] is a valid `ZeroableUlid`.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::{Ulid, ZeroableUlid};
    ///
    /// let u1 = Ulid::generate();
    /// let u2 = ZeroableUlid::from_ulid(u1);
    ///
    /// assert!(!u2.is_zero());
    /// assert_eq!(u1.to_u128(), u2.to_u128());
    /// ```
    #[must_use]
    pub const fn from_ulid(ulid: Ulid) -> Self {
        Self::from_u128(ulid.to_u128())
    }

    /// Returns the timestamp and randomness parts of a `ZeroableUlid` as a pair.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u = ZeroableUlid::generate();
    /// let (timestamp, randomness) = u.to_parts();
    ///
    /// assert_eq!(timestamp, u.timestamp());
    /// assert_eq!(randomness, u.randomness());
    /// ```
    #[must_use]
    pub const fn to_parts(self) -> (u64, u128) {
        (self.timestamp(), self.randomness())
    }

    /// Creates a `ZeroableUlid` from a timestamp and randomness parts.
    ///
    /// # Errors
    ///
    /// Will fail if the timestamp (48 bits) or randomness (80 bits) are out of range.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u1 = ZeroableUlid::generate();
    /// let (timestamp, randomness) = u1.to_parts();
    /// let u2 = ZeroableUlid::from_parts(timestamp, randomness)?;
    ///
    /// assert_eq!(u1, u2);
    ///
    /// assert_eq!(ZeroableUlid::from_parts(0, 0)?, ZeroableUlid::new());
    /// # Ok(()) }
    /// ```
    pub const fn from_parts(timestamp: u64, randomness: u128) -> Result<Self, Error> {
        match util::from_parts(timestamp, randomness) {
            Ok(n) => Ok(Self::from_u128(n)),
            Err(err) => Err(err),
        }
    }

    /// Converts a `ZeroableUlid` into binary bytes
    ///
    /// The bytes are in network byte order (big endian).
    ///
    /// # Example
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use mr_ulid::ZeroableUlid;
    ///
    // cspell:disable-next-line
    /// let u: ZeroableUlid = "01JB05JV6H9ZA2YQ6X3K1DAGVA".parse()?;
    ///
    /// assert_eq!(u.to_bytes(), [1, 146, 192, 89, 108, 209, 79, 212, 47, 92, 221, 28, 194, 213, 67, 106]);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.to_be_bytes()
    }

    /// Creates a `ZeroableUlid` from a binary byte array.
    ///
    /// The byte array must be in network byte order (big endian).
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let bytes: [u8; 16] = [1, 146, 192, 89, 108, 209, 79, 212, 47, 92, 221, 28, 194, 213, 67, 106];
    /// let u = ZeroableUlid::from_bytes(bytes);
    ///
    // cspell:disable-next-line
    /// assert_eq!(u.to_string(), "01JB05JV6H9ZA2YQ6X3K1DAGVA");
    /// ```
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self::from_u128(u128::from_be_bytes(bytes))
    }

    /// Converts a `ZeroableUlid` into a `u128` integer.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use mr_ulid::ZeroableUlid;
    ///
    // cspell:disable-next-line
    /// let u: ZeroableUlid = "01JB07NQ643XZXVHZDY0JNYR02".parse()?;
    ///
    /// assert_eq!(u.to_u128(), 2091207293934528941058695985186693122);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub const fn to_u128(self) -> u128 {
        self.0
    }

    /// Creates a `ZeroableUlid` from a `u128` integer.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let n = 2091207293934528941058695985186693122_u128;
    /// let u = ZeroableUlid::from_u128(n);
    ///
    // cspell:disable-next-line
    /// assert_eq!(u.to_string(), "01JB07NQ643XZXVHZDY0JNYR02");
    /// ```
    #[must_use]
    pub const fn from_u128(n: u128) -> Self {
        Self(n)
    }

    /// Generates a new `ZeroableUlid` and never panics.
    ///
    /// This is a variant of [`ZeroableUlid::generate()`] which never panics (with the [`STANDARD_ENTROPY_SOURCE`](generator::STANDARD_ENTROPY_SOURCE)).
    ///
    /// In the case of problems with the ULID-generator, this function returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # { inner(); fn inner() -> Option<()> {
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u1 = ZeroableUlid::try_generate()?;
    /// let u2 = ZeroableUlid::try_generate()?;
    ///
    /// assert!(u1 != u2);
    /// assert!(u1.timestamp() <= u2.timestamp());
    /// # Some(()) }}
    /// ```
    #[must_use]
    pub fn try_generate() -> Option<Self> {
        Some(Self(generator::generate()?))
    }

    /// Returns the timestamp part of a `ZeroableUlid` as a [`SystemTime`] and never panics.
    ///
    /// This is a variant of [`ZeroableUlid::datetime()`] which never panics.
    ///
    /// In the case that the timestamp of a [`ZeroableUlid`] cannot be encoded in a [`SystemTime`], this method returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::SystemTime;
    /// use mr_ulid::ZeroableUlid;
    ///
    /// let u = ZeroableUlid::generate();
    ///
    /// let datetime: Option<SystemTime> = u.try_datetime();
    /// ```
    #[must_use]
    pub fn try_datetime(self) -> Option<SystemTime> {
        SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(self.timestamp()))
    }

    /// Return the string representation of a `ZeroableUlid` and never panics.
    ///
    /// While the blanket implementation of [`std::string::ToString`] for `std::fmt::Display` may
    /// panic, this method is guaranteed to never panic, but returns `None` if the string representation cannot be created.
    /// One reason this can happen is if the allocation of memory for the string fails.
    #[must_use]
    pub fn try_to_string(self) -> Option<String> {
        util::try_to_string(self.0)
    }

    /// Creates a `ZeroableUlid` from timestamp and randomness parts without checking.
    ///
    /// This results in undefined behaviour if timestamp or randomness parts are to large
    /// or when both of them are zero.
    ///
    /// # Safety
    ///
    /// - Timestamp must less than 2<sup>48</sup>.
    /// - Randomness must less than 2<sup>80</sup>.
    #[must_use]
    pub const unsafe fn from_parts_unchecked(timestamp: u64, randomness: u128) -> Self {
        Self(((timestamp as u128) << RANDOM_BITS) | randomness)
    }
}

impl fmt::Debug for ZeroableUlid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        util::debug_ulid("ZeroableUlid", self.0, f)
    }
}

impl fmt::Display for ZeroableUlid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = [0; 26];
        f.write_str(base32::encode(self.0, &mut buffer))
    }
}

impl FromStr for ZeroableUlid {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let buffer = util::as_array(s.as_bytes())?;
        Ok(Self::from_u128(base32::decode(buffer)?))
    }
}

impl From<Ulid> for ZeroableUlid {
    fn from(non_zero: Ulid) -> Self {
        Self::from_u128(non_zero.to_u128())
    }
}

impl From<ZeroableUlid> for u128 {
    fn from(ulid: ZeroableUlid) -> Self {
        ulid.to_u128()
    }
}

impl From<u128> for ZeroableUlid {
    fn from(n: u128) -> Self {
        Self::from_u128(n)
    }
}

impl From<ZeroableUlid> for [u8; 16] {
    fn from(ulid: ZeroableUlid) -> Self {
        ulid.to_bytes()
    }
}

impl From<[u8; 16]> for ZeroableUlid {
    fn from(bytes: [u8; 16]) -> Self {
        Self::from_bytes(bytes)
    }
}

impl From<&[u8; 16]> for ZeroableUlid {
    fn from(bytes: &[u8; 16]) -> Self {
        Self::from_bytes(*bytes)
    }
}

impl TryFrom<&[u8]> for ZeroableUlid {
    type Error = Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Ok(Self::from_bytes(*util::as_array(bytes)?))
    }
}
