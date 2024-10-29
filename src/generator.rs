use std::{ops::RangeInclusive, ptr::null, sync::Mutex};
#[cfg(feature = "rand")]
use std::{ptr::addr_of_mut, time::SystemTime};

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

// # Safety
//
// It is guaranteed, that all `timestamp` and `random` functions
// of all existing `RawEntropySource` structs are never called concurrently.
// However, it is *not* guaranteed, that the `drop` callbacks of different
// `RawEntropySource` structs are never called concurrently.
// Of course it is guaranteed that after `drop` was called neither `timestamp`
// nor `random` functions are called, and that `drop` is not called concurrently
// to them.
#[derive(PartialEq, Eq)]
pub struct RawEntropySource {
    pub data: *const (),
    pub timestamp: unsafe fn(data: *const ()) -> Option<u64>,
    pub random: unsafe fn(data: *const (), range: RangeInclusive<u128>) -> Option<u128>,
    pub drop: unsafe fn(data: *const ()),
}

unsafe impl Send for RawEntropySource {}

/// An opaque handle for entropy sources.
///
/// A `EntropySourceHandle` wraps different types of entropy sources:
///
/// - [`NO_ENTROPY_SOURCE`]
/// - [`STANDARD_ENTROPY_SOURCE`]
/// - Types implementing the [`EntropySource`] trait.
// - [`RawEntropySource`]
///
/// A `EntropySourceHandle` is accepted by [`set_entropy_source`] function.
///
// # Safety
//
// `EntropySourceHandle`
//   - owns the underlying `RawEntropySource`,
//   - calls the drop callback of the underlying `RawEntropySource` when dropped,
//   - does not call any other callbacks to the underlying `RawEntropySource`,
//   - is only used by the `Generator` struct,
//   - only one `Generator` exists (behind a `Mutex`).
//
// So it is ensured, that the `EntropySourceHandle` is not accessed concurrently by multiple threads.
#[repr(transparent)]
pub struct EntropySourceHandle {
    raw: RawEntropySource,
}

impl EntropySourceHandle {
    /// Creates an `EntropySourceHandle` from a type implementing the `EntropySource` trait.
    #[must_use]
    pub fn new<T: EntropySource>(source: T) -> Self {
        let data = Box::into_raw(Box::new(source)) as *const ();

        let timestamp = |data| unsafe {
            let source = data as *mut T;
            (*source).timestamp()
        };

        let random = |data, range| unsafe {
            let source = data as *mut T;
            (*source).random(range)
        };

        let drop = |data| unsafe {
            let _ = Box::<T>::from_raw(data as *mut T);
        };

        Self::from_raw(RawEntropySource {
            data,
            timestamp,
            random,
            drop,
        })
    }

    /// Creates a new `EntropySourceHandle` from a given `RawEntropySource`.
    #[must_use]
    const fn from_raw(raw: RawEntropySource) -> Self {
        Self { raw }
    }

    /// # Safety
    ///
    /// Only `Generator::generate()` is allowed to call this method.
    #[allow(clippy::needless_pass_by_ref_mut)]
    #[must_use]
    unsafe fn random(&mut self, range: RangeInclusive<u128>) -> Option<u128> {
        // TODO: Once Rust 2024 arrives, `RangeInclusive` should be `Copy`, so remove `clone()` then.
        let candidate = unsafe { (self.raw.random)(self.raw.data, range.clone()) }?;

        // A small step for the CPU, a huge step for resilience...
        range.contains(&candidate).then_some(candidate)
    }

    /// # Safety
    ///
    /// Only `Generator::generate()` is allowed to call this method.
    #[allow(clippy::needless_pass_by_ref_mut)]
    #[must_use]
    unsafe fn timestamp(&mut self) -> Option<u64> {
        let candidate = unsafe { (self.raw.timestamp)(self.raw.data) }?;

        // The last possible millisecond (TIMESTAMP_MAX) is reserved four our guarantees.
        (candidate < TIMESTAMP_MAX).then_some(candidate)
    }
}

impl Drop for EntropySourceHandle {
    fn drop(&mut self) {
        unsafe { (self.raw.drop)(self.raw.data) }
    }
}

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
pub const NO_ENTROPY_SOURCE: EntropySourceHandle = EntropySourceHandle::from_raw(RawEntropySource {
    data: null(),
    timestamp: |_| None,
    random: |_, _| None,
    drop: |_| {},
});

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
pub const STANDARD_ENTROPY_SOURCE: EntropySourceHandle = {
    static mut RNG: Option<StdRng> = None;

    let timestamp = |_data| {
        let now = SystemTime::now();
        let since_epoch = now.duration_since(SystemTime::UNIX_EPOCH).ok()?;
        let millis = since_epoch.as_millis();
        u64::try_from(millis).ok()
    };

    let random = |_data, range| unsafe {
        let rng = (*addr_of_mut!(RNG)).get_or_insert_with(StdRng::from_entropy);
        Some(rng.gen_range(range))
    };

    EntropySourceHandle::from_raw(RawEntropySource {
        data: null(),
        timestamp,
        random,
        drop: |_| {},
    })
};

struct Generator {
    entropy: EntropySourceHandle,
    last_ulid: u128,
}

impl Generator {
    // # Safety
    //
    // Only the function `generate()` below in this module,
    // is allowed to call this method.
    unsafe fn generate(&mut self) -> Option<u128> {
        // SAFETY: This Method has the exclusive permission to call `timestamp()`
        let now = unsafe { self.entropy.timestamp() }?;

        assert!(now < TIMESTAMP_MAX); // Yes, smaller, *not* smaller or equal!

        let timestamp = u128::from(now) << RANDOM_BITS;
        let last_timestamp = self.last_ulid & TIMESTAMP_MASK;

        let ulid = if timestamp > last_timestamp {
            // Ensure ULID is always non-zero, regardless of timestamp
            // SAFETY: This Method has the exclusive permission to call `random()`
            let random = unsafe { self.entropy.random(1..=RANDOM_GEN_MAX)? };
            timestamp | random
        } else {
            self.last_ulid.checked_add(1)?
        };

        assert!(ulid > self.last_ulid);

        self.last_ulid = ulid;

        Some(ulid)
    }
}

static GENERATOR: Mutex<Generator> = {
    #[cfg(feature = "rand")]
    let entropy = STANDARD_ENTROPY_SOURCE;

    #[cfg(not(feature = "rand"))]
    let entropy = NO_ENTROPY_SOURCE;

    let generator = Generator { entropy, last_ulid: 0 };

    Mutex::new(generator)
};

// # Safety
//
// - Only this function is allowed to call `Generator::generate()`.
// - Only `Generator::generate()` is allowed to call the methods
//   `EntropySourceHandle::timestamp()` and `EntropySourceHandle::random()`.
// - The access to `Generator::generate()` is protected by a Mutex.
//
// So we ensure that all callback functions are not called concurrently.
pub fn generate() -> Option<u128> {
    let mut generator = GENERATOR.lock().ok()?;
    unsafe { generator.generate() }
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

    std::mem::replace(&mut generator.entropy, source)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Ulid;

    fn manipulate_generator_last_ulid(last_id: u128) {
        let mut generator = GENERATOR.lock().unwrap();
        generator.last_ulid = last_id;
    }

    struct BackupRestorer {
        source: Option<EntropySourceHandle>,
        last_ulid: u128,
    }
    impl BackupRestorer {
        #[must_use]
        fn new() -> Self {
            let mut generator = GENERATOR.lock().unwrap();
            let source = std::mem::replace(&mut generator.entropy, NO_ENTROPY_SOURCE);
            Self {
                source: Some(source),
                last_ulid: generator.last_ulid,
            }
        }
    }
    impl Drop for BackupRestorer {
        fn drop(&mut self) {
            let mut generator = GENERATOR.lock().unwrap();
            generator.entropy = self.source.take().unwrap();
            generator.last_ulid = self.last_ulid;
        }
    }

    struct FixedEntropySource {
        timestamp: u64,
        random: u128,
    }
    impl FixedEntropySource {
        #[must_use]
        const fn new(timestamp: u64, random: u128) -> Self {
            Self { timestamp, random }
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
        let _backup = BackupRestorer::new();

        let source = FixedEntropySource::new(1, 1);
        let handle = EntropySourceHandle::new(source);
        set_entropy_source(handle);

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
}
