use std::{
    fmt,
    num::NonZero,
    str::FromStr,
    time::{Duration, SystemTime},
};

use crate::{Error, RANDOM_BITS, RANDOM_MASK, ZeroableUlid, base32, generator, util};

/// A ULID which never is zero.
///
/// Because this `Ulid` can never become zero, size of `Ulid` and size of `Option<Ulid>` are
/// guaranteed to be equal thanks to Rust null pointer optimization:
///
/// ```
/// use std::mem::size_of;
/// use mr_ulid::Ulid;
///
/// assert_eq!(size_of::<Ulid>(), size_of::<Option<Ulid>>());
/// ```
///
/// Parsing a zero value ULID will fail:
///
/// ```
/// use mr_ulid::Ulid;
///
/// let s = "00000000000000000000000000";
///
/// assert!(s.parse::<Ulid>().is_err());
/// ```
///
/// For a ULID which can become zero, check out the [`ZeroableUlid`] type.
/// However, it is more idiomatic to just use `Option<Ulid>`.
///
#[derive(Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Ulid(NonZero<u128>);

impl Ulid {
    /// Minimum allowed [`Ulid`]
    ///
    /// The smallest value for [`Ulid`] is 1, because zero is explicitly not allowed.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::Ulid;
    ///
    /// assert_eq!(Ulid::MIN.to_u128(), 1);
    /// ```
    pub const MIN: Self = unsafe { Self::from_u128_unchecked(1) };

    /// Maximum allowed [`Ulid`]
    ///
    /// The largest value for [`Ulid`] is `u128::MAX`.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::Ulid;
    ///
    /// assert_eq!(Ulid::MAX.to_u128(), u128::MAX);
    /// ```
    pub const MAX: Self = unsafe { Self::from_u128_unchecked(u128::MAX) };

    /// Generates a new unique ULID.
    ///
    /// The generated ULIDs are guaranteed to be unique and monotonically increasing and never zero.
    ///
    /// A lot of care is taken, that the ULIDs cannot overflow. You can create as many
    /// ULIDs as you like, and they will always be unique and strict monotonically increasing.
    ///
    /// # Panics
    ///
    /// With the standard entropy source ([`STANDARD_ENTROPY_SOURCE`][generator::STANDARD_ENTROPY_SOURCE]),
    /// this method will panic if the system date is somewhere after the year 10889 or before the Unix epoch (year 1970).
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::Ulid;
    ///
    /// let u1 = Ulid::new();
    /// let u2 = Ulid::new();
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
    pub fn new() -> Self {
        Self(NonZero::new(generator::generate().unwrap()).unwrap())
    }

    /// Returns the timestamp part of a `Ulid`.
    ///
    /// The timestamp is measured in milliseconds since the Unix epoch (1. January 1970).
    /// ULID timestamps are limited to 48 bits.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::Ulid;
    ///
    /// let u = Ulid::new();
    ///
    /// assert!(u.timestamp() > 1704067200000); // 1st January 2024
    /// ```
    #[must_use]
    pub const fn timestamp(self) -> u64 {
        (self.0.get() >> RANDOM_BITS) as u64
    }

    /// Returns the random part of a `Ulid`.
    ///
    /// The randomness of a `ULID` is limited to 80 bits.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::Ulid;
    ///
    /// let u = Ulid::new();
    ///
    /// assert!(u.randomness() < (1<<80));
    /// ```
    ///
    #[must_use]
    pub const fn randomness(self) -> u128 {
        self.0.get() & RANDOM_MASK
    }

    /// Returns the timestamp part of a `Ulid` as a `SystemTime`.
    ///
    /// # Panics
    ///
    /// In Rust the allowed range for [`SystemTime`] is not defined.
    /// So this method may panic if the timestamp of the ULID cannot represented with [`SystemTime`].
    /// On most common systems that will not happen.
    ///
    /// For a variant which never panics, see [`Ulid::try_datetime`].
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::SystemTime;
    /// use mr_ulid::Ulid;
    ///
    /// let u = Ulid::new();
    ///
    /// assert!(u.datetime() <= SystemTime::now());
    /// ```
    #[must_use]
    pub fn datetime(self) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_millis(self.timestamp())
    }

    /// Converts this `Ulid` to a [`ZeroableUlid`].
    ///
    /// This method always succeeds, as every [`Ulid`] is a valid [`ZeroableUlid`].
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::Ulid;
    ///
    /// let u1 = Ulid::new();
    /// let u2 = u1.to_zeroable_ulid();
    ///
    /// assert_eq!(u1.to_u128(), u2.to_u128());
    ///
    /// ```
    #[must_use]
    pub const fn to_zeroable_ulid(self) -> ZeroableUlid {
        ZeroableUlid::from_u128(self.0.get())
    }

    /// Creates a `Ulid` from a [`ZeroableUlid`].
    ///
    /// When the [`ZeroableUlid`] is zero, `None` is returned.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::{Ulid, ZeroableUlid};
    ///
    /// let u1 = ZeroableUlid::new();
    /// assert!(Ulid::from_zeroable_ulid(u1).is_some());
    ///
    /// let u2 = ZeroableUlid::zeroed(); // Create a ZeroableUlid with zero value
    /// assert!(Ulid::from_zeroable_ulid(u2).is_none());
    /// ```
    #[must_use]
    pub const fn from_zeroable_ulid(zeroable: ZeroableUlid) -> Option<Self> {
        Self::from_u128(zeroable.to_u128())
    }

    /// Returns the timestamp and randomness parts of a `Ulid` as a pair.
    ///
    /// # Example
    ///
    /// ```
    /// use mr_ulid::Ulid;
    ///
    /// let u = Ulid::new();
    /// let (timestamp, randomness) = u.to_parts();
    ///
    /// assert_eq!(timestamp, u.timestamp());
    /// assert_eq!(randomness, u.randomness());
    /// ```
    #[must_use]
    pub const fn to_parts(self) -> (u64, u128) {
        (self.timestamp(), self.randomness())
    }

    /// Creates a `Ulid` from a timestamp and randomness parts.
    ///
    /// # Errors
    ///
    /// Will fail if the timestamp (48 bits) or randomness (80 bits) are out of range,
    /// and will fail, if both values are zero, because [`Ulid`] is not allowed to be zero.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use mr_ulid::Ulid;
    ///
    /// let u1 = Ulid::new();
    /// let (timestamp, randomness) = u1.to_parts();
    /// let u2 = Ulid::from_parts(timestamp, randomness)?;
    ///
    /// assert_eq!(u1, u2);
    ///
    /// assert!(Ulid::from_parts(0, 0).is_err());
    /// # Ok(()) }
    /// ```
    pub const fn from_parts(timestamp: u64, randomness: u128) -> Result<Self, Error> {
        match util::from_parts(timestamp, randomness) {
            Ok(n) => match Self::from_u128(n) {
                Some(ulid) => Ok(ulid),
                None => Err(Error::InvalidZero),
            },
            Err(error) => Err(error),
        }
    }

    /// Converts a `Ulid` into binary bytes
    ///
    /// The bytes are in network byte order (big endian).
    ///
    /// # Example
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use mr_ulid::Ulid;
    ///
    // cspell:disable-next-line
    /// let ulid: Ulid = "01JB05JV6H9ZA2YQ6X3K1DAGVA".parse()?;
    ///
    /// assert_eq!(ulid.to_bytes(), [1, 146, 192, 89, 108, 209, 79, 212, 47, 92, 221, 28, 194, 213, 67, 106]);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0.get().to_be_bytes()
    }

    /// Creates a `Ulid` from a binary byte array.
    ///
    /// The byte array must be in network byte order (big endian).
    ///
    /// Returns `None` if all bytes in the byte array are zero, because [`Ulid`] ist not allowed to be zero,
    ///
    /// # Example
    ///
    /// ```
    /// # { inner(); fn inner() -> Option<()> {
    /// use mr_ulid::Ulid;
    ///
    /// let bytes: [u8; 16] = [1, 146, 192, 89, 108, 209, 79, 212, 47, 92, 221, 28, 194, 213, 67, 106];
    /// let u = Ulid::from_bytes(bytes)?;
    ///
    // cspell:disable-next-line
    /// assert_eq!(u.to_string(), "01JB05JV6H9ZA2YQ6X3K1DAGVA");
    /// # Some(()) }}
    /// ```
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Option<Self> {
        Self::from_u128(u128::from_be_bytes(bytes))
    }

    /// Converts a `Ulid` into a `u128` integer.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use mr_ulid::Ulid;
    ///
    // cspell:disable-next-line
    /// let u: Ulid = "01JB07NQ643XZXVHZDY0JNYR02".parse()?;
    ///
    /// assert_eq!(u.to_u128(), 2091207293934528941058695985186693122);
    /// # Ok(()) }
    /// ```
    #[must_use]
    pub const fn to_u128(self) -> u128 {
        self.0.get()
    }

    /// Creates a `Ulid` from a `u128` integer.
    ///
    /// # Errors
    ///
    /// Return `None` if the value was zero, because [`Ulid`] is not allowed to be zero.
    ///
    /// # Example
    ///
    /// ```
    /// # { inner(); fn inner() -> Option<()> {
    /// use mr_ulid::Ulid;
    ///
    /// let n = 2091207293934528941058695985186693122_u128;
    /// let u = Ulid::from_u128(n)?;
    ///
    // cspell:disable-next-line
    /// assert_eq!(u.to_string(), "01JB07NQ643XZXVHZDY0JNYR02");
    /// # Some(()) }}
    /// ```
    #[must_use]
    pub const fn from_u128(n: u128) -> Option<Self> {
        match NonZero::new(n) {
            Some(non_zero) => Some(Self(non_zero)),
            None => None,
        }
    }

    /// Converts a `Ulid` into a `NonZero<u128>` integer.
    ///
    /// # Example
    ///
    /// ```
    /// # { inner(); fn inner() -> Option<()> {
    /// use std::num::NonZero;
    ///
    /// use mr_ulid::Ulid;
    ///
    // cspell:disable-next-line
    /// let u = Ulid::from_u128(42)?;
    ///
    /// assert_eq!(u.to_non_zero_u128(), NonZero::new(42)?);
    /// # Some(()) }}
    /// ```
    #[must_use]
    pub const fn to_non_zero_u128(self) -> NonZero<u128> {
        self.0
    }

    /// Creates a `Ulid` from a `NonZero<u128>` integer.
    ///
    /// Because the `NonZero<u128>` integer cannot be zero, this method always succeed.
    ///
    /// # Example
    ///
    /// ```
    /// # { inner(); fn inner() -> Option<()> {
    /// use std::num::NonZero;
    ///
    /// use mr_ulid::Ulid;
    ///
    /// let n = NonZero::new(2091207293934528941058695985186693122)?;
    /// let u = Ulid::from_non_zero_u128(n);
    ///
    // cspell:disable-next-line
    /// assert_eq!(u.to_string(), "01JB07NQ643XZXVHZDY0JNYR02");
    /// # Some(()) }}
    /// ```
    #[must_use]
    pub const fn from_non_zero_u128(non_zero: NonZero<u128>) -> Self {
        Self(non_zero)
    }

    /// Generates a new `Ulid` and never panics.
    ///
    /// This is a variant of [`Ulid::new()`] which never panics (with the [`STANDARD_ENTROPY_SOURCE`](generator::STANDARD_ENTROPY_SOURCE)).
    ///
    /// In the case of problems with the ULID-generator, this function returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # { inner(); fn inner() -> Option<()> {
    /// use mr_ulid::Ulid;
    ///
    /// let u1 = Ulid::try_new()?;
    /// let u2 = Ulid::try_new()?;
    ///
    /// assert!(u1 != u2);
    /// assert!(u1.timestamp() <= u2.timestamp());
    /// # Some(()) }}
    /// ```
    #[must_use]
    pub fn try_new() -> Option<Self> {
        Some(Self(NonZero::new(generator::generate()?)?))
    }

    /// Returns the timestamp part of a `Ulid` as a [`SystemTime`] and never panics.
    ///
    /// This is a variant of [`Ulid::datetime()`] which never panics.
    ///
    /// In the case that the timestamp of a [`Ulid`] cannot be encoded in a [`SystemTime`], this method returns `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use std::time::SystemTime;
    /// use mr_ulid::Ulid;
    ///
    /// let u = Ulid::new();
    ///
    /// let datetime: Option<SystemTime> = u.try_datetime();
    /// ```
    #[must_use]
    pub fn try_datetime(self) -> Option<SystemTime> {
        SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(self.timestamp()))
    }

    /// Return the string representation of a [`Ulid`] and never panics.
    ///
    /// While the blanket implementation of [`std::string::ToString`] for `std::fmt::Display` may
    /// panic, this method is guaranteed to never panic, but returns `None` if the string representation cannot be created.
    /// One reason this can happen is if the allocation of memory for the string fails.
    #[must_use]
    pub fn try_to_string(self) -> Option<String> {
        util::try_to_string(self.0.get())
    }

    /// Creates a `Ulid` from timestamp and randomness parts without checking.
    ///
    /// This results in undefined behaviour if timestamp or randomness parts are to large
    /// or when both of them are zero.
    ///
    /// # Safety
    ///
    /// - Timestamp must less than 2<sup>48</sup>.
    /// - Randomness must less than 2<sup>80</sup>.
    /// - One part (timestamp or randomness) must be non-zero.
    #[must_use]
    pub const unsafe fn from_parts_unchecked(timestamp: u64, randomness: u128) -> Self {
        let n = ((timestamp as u128) << RANDOM_BITS) | randomness;
        Self(unsafe { NonZero::new_unchecked(n) })
    }

    /// Creates a `Ulid` from a `u128` integer without checking whether the value is zero.
    ///
    /// This results in undefined behaviour if the value is zero.
    ///
    /// # Safety
    ///
    /// The value must not be zero.
    #[must_use]
    pub const unsafe fn from_u128_unchecked(n: u128) -> Self {
        Self(unsafe { NonZero::new_unchecked(n) })
    }

    /// Creates a `Ulid` from a binary byte array without checking whether at least one byte is non-zero.
    ///
    /// This results in undefined behaviour if all bytes are zero.
    ///
    /// # Safety
    ///
    /// At least one byte must be non-zero.
    #[must_use]
    pub const unsafe fn from_bytes_unchecked(bytes: [u8; 16]) -> Self {
        let n = u128::from_be_bytes(bytes);
        Self(unsafe { NonZero::new_unchecked(n) })
    }
}

impl Default for Ulid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        util::debug_ulid("Ulid", self.0.get(), f)
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buffer = [0; 26];
        f.write_str(base32::encode(self.0.get(), &mut buffer))
    }
}

impl FromStr for Ulid {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let buffer = util::as_array(s.as_bytes())?;
        Self::from_u128(base32::decode(buffer)?).ok_or(Error::InvalidZero)
    }
}

impl TryFrom<ZeroableUlid> for Ulid {
    type Error = Error;
    fn try_from(zeroable: ZeroableUlid) -> Result<Self, Self::Error> {
        Self::from_u128(zeroable.to_u128()).ok_or(Error::InvalidZero)
    }
}

impl From<Ulid> for u128 {
    fn from(ulid: Ulid) -> Self {
        ulid.to_u128()
    }
}

impl TryFrom<u128> for Ulid {
    type Error = Error;
    fn try_from(n: u128) -> Result<Self, Self::Error> {
        Self::from_u128(n).ok_or(Error::InvalidZero)
    }
}

impl From<Ulid> for NonZero<u128> {
    fn from(ulid: Ulid) -> Self {
        ulid.to_non_zero_u128()
    }
}

impl From<NonZero<u128>> for Ulid {
    fn from(non_zero: NonZero<u128>) -> Self {
        Self::from_non_zero_u128(non_zero)
    }
}

impl From<Ulid> for [u8; 16] {
    fn from(ulid: Ulid) -> Self {
        ulid.to_bytes()
    }
}

impl TryFrom<[u8; 16]> for Ulid {
    type Error = Error;
    fn try_from(bytes: [u8; 16]) -> Result<Self, Self::Error> {
        Self::from_bytes(bytes).ok_or(Error::InvalidZero)
    }
}

impl TryFrom<&[u8; 16]> for Ulid {
    type Error = Error;
    fn try_from(bytes: &[u8; 16]) -> Result<Self, Self::Error> {
        Self::from_bytes(*bytes).ok_or(Error::InvalidZero)
    }
}

impl TryFrom<&[u8]> for Ulid {
    type Error = Error;
    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        Self::from_bytes(*util::as_array(bytes)?).ok_or(Error::InvalidZero)
    }
}
