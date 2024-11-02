#[cfg(feature = "rand")]
use std::time::SystemTime;
use std::{fmt, ops::RangeInclusive, sync::Mutex};

#[cfg(feature = "rand")]
use rand::{rngs::StdRng, Rng as _, SeedableRng as _}; // cspell:disable-line

use crate::{RANDOM_BITS, RANDOM_GEN_MAX, TIMESTAMP_MASK, TIMESTAMP_MAX};

/// Trait for entropy sources.
///
/// For a type to be used as an entropy source, implement the `EntropySource` trait and
/// create an [`EntropySourceHandle`] out of it and set the handle using the [`set_entropy_source`] function.
///
/// # Example
///
/// ```no_run
/// struct MySource;
///
/// impl MySource {
///     fn new() -> Self {
///         todo!()
///     }
/// }
///
/// impl mr_ulid::EntropySource for MySource {
///     fn timestamp(&mut self) -> Option<u64> {
///         todo!()
///     }
///     fn random(&mut self, range: std::ops::RangeInclusive<u128>) -> Option<u128> {
///         todo!()
///     }
/// }
///
/// let my_source = MySource::new();
/// let handle = mr_ulid::EntropySourceHandle::new(my_source);
///
/// mr_ulid::set_entropy_source(handle);
/// ```
pub trait EntropySource: Send {
    /// Returns the current timestamp in milliseconds since the Unix epoch.
    fn timestamp(&mut self) -> Option<u64>;

    /// Returns a random number in the given range.
    fn random(&mut self, range: RangeInclusive<u128>) -> Option<u128>;
}

/// An opaque handle for entropy sources.
///
/// A `EntropySourceHandle` wraps different types of entropy sources:
///
/// - [`NO_ENTROPY_SOURCE`]
/// - [`STANDARD_ENTROPY_SOURCE`]
/// - Types implementing the [`EntropySource`] trait.
///
/// A `EntropySourceHandle` is accepted by [`set_entropy_source`] function.
///
#[repr(transparent)]
pub struct EntropySourceHandle {
    inner: InnerHandle,
}

enum InnerHandle {
    NoOp,
    #[cfg(feature = "rand")]
    Standard,
    Custom(Box<dyn EntropySource>),
}

impl EntropySourceHandle {
    /// Creates an `EntropySourceHandle` from a type implementing the `EntropySource` trait.
    #[must_use]
    pub fn new(source: impl EntropySource + 'static) -> Self {
        Self {
            inner: InnerHandle::Custom(Box::new(source)),
        }
    }
}

impl fmt::Debug for EntropySourceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("EntropySourceHandle { ... }")
    }
}

/// Standard entropy source.
///
/// This is the default entropy source used to generate ULIDs if no
/// other entropy source is set.
///
/// This entropy source uses the system clock and the `rand` crate. So if
/// the `rand` crate is disabled, this entropy source is not available.
///
/// # Example
///
/// ```
/// use mr_ulid::Ulid;
///
/// mr_ulid::set_entropy_source(mr_ulid::STANDARD_ENTROPY_SOURCE);
///
/// assert!(Ulid::try_generate().is_some());
/// ```
#[cfg(feature = "rand")]
pub const STANDARD_ENTROPY_SOURCE: EntropySourceHandle = EntropySourceHandle {
    inner: InnerHandle::Standard,
};

/// No-Operation entropy source.
///
/// An entropy source which never generates any timestamps nor any random values.
/// Setting this source will result in no ULIDs generated:
///
/// - `Ulid::try_generate()` will always return `None`.
/// - `Ulid::generate()` will panic.
///
/// This entropy source is the default source if the `rand` crate is not enabled.
///
/// # Example
///
/// ```no_run
/// use mr_ulid::Ulid;
///
/// mr_ulid::set_entropy_source(mr_ulid::NO_ENTROPY_SOURCE);
///
/// assert_eq!(Ulid::try_generate(), None);
/// ```
pub const NO_ENTROPY_SOURCE: EntropySourceHandle = EntropySourceHandle {
    inner: InnerHandle::NoOp,
};

struct Generator {
    source: EntropySourceHandle,
    #[cfg(feature = "rand")]
    rng: Option<StdRng>,
    last_ulid: u128,
}

impl Generator {
    #[must_use]
    fn generate(&mut self) -> Option<u128> {
        let now = self.timestamp()?;
        assert!(now < TIMESTAMP_MAX); // Yes, smaller, *not* smaller or equal!

        let timestamp = u128::from(now) << RANDOM_BITS;
        let last_timestamp = self.last_ulid & TIMESTAMP_MASK;

        let ulid = if timestamp > last_timestamp {
            // Ensure ULID is always non-zero, regardless of timestamp
            let random = self.random(1..=RANDOM_GEN_MAX)?;
            timestamp | random
        } else {
            self.last_ulid.checked_add(1)?
        };

        assert!(ulid > self.last_ulid);

        self.last_ulid = ulid;

        Some(ulid)
    }

    #[must_use]
    fn timestamp(&mut self) -> Option<u64> {
        let candidate = match &mut self.source.inner {
            InnerHandle::NoOp => None,
            #[cfg(feature = "rand")]
            InnerHandle::Standard => {
                let now = SystemTime::now();
                let since_epoch = now.duration_since(SystemTime::UNIX_EPOCH).ok()?;
                let millis = since_epoch.as_millis();
                u64::try_from(millis).ok()
            }
            InnerHandle::Custom(source) => source.timestamp(),
        }?;

        // The last possible millisecond (TIMESTAMP_MAX) is reserved four our guarantees.
        (candidate < TIMESTAMP_MAX).then_some(candidate)
    }

    #[must_use]
    fn random(&mut self, range: RangeInclusive<u128>) -> Option<u128> {
        let candidate = match &mut self.source.inner {
            InnerHandle::NoOp => None,
            #[cfg(feature = "rand")]
            InnerHandle::Standard => {
                let rng = self.rng.get_or_insert_with(StdRng::from_entropy);
                // TODO: Once Rust 2024 arrives, `RangeInclusive` should be `Copy`, so remove `clone()` then.
                Some(rng.gen_range(range.clone()))
            }
            InnerHandle::Custom(source) => {
                // TODO: dito
                source.random(range.clone())
            }
        }?;

        // A small step for the CPU, a huge step for resilience...
        range.contains(&candidate).then_some(candidate)
    }
}

static GENERATOR: Mutex<Generator> = {
    #[cfg(feature = "rand")]
    let generator = Generator {
        source: STANDARD_ENTROPY_SOURCE,
        rng: None,
        last_ulid: 0,
    };

    #[cfg(not(feature = "rand"))]
    let generator = Generator {
        source: NO_ENTROPY_SOURCE,
        last_ulid: 0,
    };

    Mutex::new(generator)
};

pub fn generate() -> Option<u128> {
    let mut generator = GENERATOR.lock().ok()?;
    generator.generate()
}

/// Sets the entropy source for generating ULIDs.
///
/// Sets a new entropy source and returns the previous set entropy source.
///
/// Normally you don't need to call this function unless the `rand` crate is disabled,
/// or if you're using a custom entropy source.
pub fn set_entropy_source(source: EntropySourceHandle) -> EntropySourceHandle {
    let mut generator = GENERATOR.lock().unwrap_or_else(|poisoned| {
        GENERATOR.clear_poison();
        poisoned.into_inner()
    });

    std::mem::replace(&mut generator.source, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ulid;

    fn manipulate_generator_last_ulid(last_id: u128) {
        let mut generator = GENERATOR.lock().unwrap();
        generator.last_ulid = last_id;
    }

    struct RestoreStandardSource;
    impl Drop for RestoreStandardSource {
        fn drop(&mut self) {
            set_entropy_source(STANDARD_ENTROPY_SOURCE);
            manipulate_generator_last_ulid(0);
        }
    }

    struct FixedEntropySource {
        timestamp: u64,
        random: u128,
    }
    impl FixedEntropySource {
        fn install(timestamp: u64, random: u128) -> RestoreStandardSource {
            let source = Self { timestamp, random };
            let handle = EntropySourceHandle::new(source);
            set_entropy_source(handle);
            RestoreStandardSource
        }
    }
    impl EntropySource for FixedEntropySource {
        fn timestamp(&mut self) -> Option<u64> {
            Some(self.timestamp)
        }
        fn random(&mut self, _range: RangeInclusive<u128>) -> Option<u128> {
            Some(self.random)
        }
    }

    #[test]
    fn test_generator_overflow() {
        let _restore = FixedEntropySource::install(1, 1);

        let u1 = Ulid::generate();
        assert_eq!(u1.timestamp(), 1);
        assert_eq!(u1.randomness(), 1);

        let u2 = Ulid::generate();
        assert_eq!(u2.timestamp(), 1);
        assert_eq!(u2.randomness(), 2);

        manipulate_generator_last_ulid((1 << RANDOM_BITS) | ((1 << RANDOM_BITS) - 2));

        let u3 = Ulid::generate();
        assert_eq!(u3.timestamp(), 1);
        assert_eq!(u3.randomness(), (1 << RANDOM_BITS) - 1);

        let u4 = Ulid::generate();
        assert_eq!(u4.timestamp(), 2);
        assert_eq!(u4.randomness(), 0);

        let u5 = Ulid::generate();
        assert_eq!(u5.timestamp(), 2);
        assert_eq!(u5.randomness(), 1);

        manipulate_generator_last_ulid(u128::MAX - 1);

        let u6 = Ulid::generate();
        assert_eq!(u6.to_u128(), u128::MAX);

        assert!(Ulid::try_generate().is_none());
    }

    #[test]
    fn test_debug() {
        struct TestSource;

        impl EntropySource for TestSource {
            fn timestamp(&mut self) -> Option<u64> {
                None
            }
            fn random(&mut self, _range: RangeInclusive<u128>) -> Option<u128> {
                None
            }
        }

        let handle = EntropySourceHandle::new(TestSource);

        assert_eq!(format!("{handle:?}"), "EntropySourceHandle { ... }");
        assert_eq!(format!("{STANDARD_ENTROPY_SOURCE:?}"), "EntropySourceHandle { ... }");
        assert_eq!(format!("{NO_ENTROPY_SOURCE:?}"), "EntropySourceHandle { ... }");
    }
}
