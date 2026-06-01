use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

use crate::*;

#[test]
fn test_sizeof() {
    assert_eq!(size_of::<Ulid>(), size_of::<u128>());
    assert_eq!(size_of::<Ulid>(), size_of::<Option<Ulid>>());

    assert_eq!(size_of::<ZeroableUlid>(), size_of::<u128>());
    assert_eq!(size_of::<Option<ZeroableUlid>>(), size_of::<Option<u128>>());
}

#[test]
const fn test_send_sync() {
    const fn assert_send<T: Send>() {}
    const fn assert_sync<T: Sync>() {}

    assert_send::<Ulid>();
    assert_sync::<Ulid>();

    assert_send::<ZeroableUlid>();
    assert_sync::<ZeroableUlid>();

    assert_send::<EntropySourceHandle>();
}

#[test]
#[cfg_attr(miri, ignore)] // miri execution is to slow
fn test_timestamp() {
    let ulid1 = Ulid::new();
    let ulid2 = Ulid::new();
    let ulid3 = Ulid::new();
    std::thread::sleep(std::time::Duration::from_millis(1));
    let ulid4 = Ulid::new();

    let ts1 = ulid1.timestamp();
    let ts2 = ulid2.timestamp();
    let ts3 = ulid3.timestamp();
    let ts4 = ulid4.timestamp();

    assert!(ts1 <= ts2);
    assert!(ts2 <= ts3);
    assert!(ts3 < ts4);

    assert!(ts1 == ts2 || ts2 == ts3);
}

#[test]
fn test_monotonicity() {
    let ulid1 = Ulid::new();
    let ulid2 = Ulid::new();
    let ulid3 = Ulid::new();

    assert!(ulid1 < ulid2);
    assert!(ulid2 < ulid3);
}

#[test]
fn test_uniques() {
    let ulid1 = Ulid::new();
    let ulid2 = Ulid::new();
    let ulid3 = Ulid::new();

    assert_ne!(ulid1, ulid2);
    assert_ne!(ulid2, ulid3);
    assert_ne!(ulid3, ulid1);
}

#[test]
fn test_parse() {
    let ulid1 = ZeroableUlid::new();
    let ulid2 = ulid1.to_string().to_lowercase().parse();
    assert_eq!(ulid2, Ok(ulid1));

    assert_eq!(
        "oooooooooooooooooooooooooo".parse::<ZeroableUlid>(),
        Ok(ZeroableUlid::zeroed())
    );

    assert_eq!(
        "zzzzzzzzzzzzzzzzzzzzzzzzzz".parse::<ZeroableUlid>(),
        Err(Error::InvalidChar)
    );

    assert_eq!("".parse::<ZeroableUlid>(), Err(Error::TooShort));

    assert_eq!(
        "1234567890123456789012345".parse::<ZeroableUlid>(),
        Err(Error::TooShort)
    );
    assert_eq!(
        "123456789012345678901234567".parse::<ZeroableUlid>(),
        Err(Error::TooLong)
    );
}

#[test]
fn test_string_length() {
    assert_eq!(Ulid::new().to_string().len(), 26);
}

#[test]
fn test_try_to_string() {
    let r1 = Ulid::new().try_to_string();
    assert!(r1.is_some());
    assert_eq!(r1.unwrap().len(), 26);

    let r2 = ZeroableUlid::new().try_to_string();
    assert!(r2.is_some());
    assert_eq!(r2.unwrap().len(), 26);
}

#[test]
fn test_debug_fmt() {
    let s = "01javee2cb2r1mp14kpoawiwiz"; // cspell:disable-line

    let ulid1: ZeroableUlid = s.parse().unwrap();
    let ulid2: Ulid = s.parse().unwrap();

    assert_eq!(
        format!("{ulid1:?}",),
        r#"ZeroableUlid { string: "01JAVEE2CB2R1MP14KP0AW1W1Z", timestamp: "2024-10-23T01:04:07.563Z", randomness: "16034B0493B015C0F03F" }"# // cspell:disable-line
    );

    assert_eq!(
        format!("{ulid2:?}",),
        r#"Ulid { string: "01JAVEE2CB2R1MP14KP0AW1W1Z", timestamp: "2024-10-23T01:04:07.563Z", randomness: "16034B0493B015C0F03F" }"# // cspell:disable-line
    );
}

#[test]
fn test_max() {
    let ulid_max = Ulid::from_u128(u128::MAX).unwrap();
    assert_eq!(ulid_max.to_string(), "7ZZZZZZZZZZZZZZZZZZZZZZZZZ");

    assert_eq!("7zzzzzzzzzzzzzzzzzzzzzzzzz".parse::<Ulid>(), Ok(ulid_max));

    assert_eq!("80000000000000000000000000".parse::<Ulid>(), Err(Error::InvalidChar));
}

#[test]
fn test_from_parts() {
    assert_eq!(Ulid::from_parts(0, 0), Err(Error::InvalidZero));

    assert_eq!(ZeroableUlid::from_parts(0, 0), Ok(ZeroableUlid::zeroed()));
    assert_eq!(ZeroableUlid::from_parts(0, 1), Ok(ZeroableUlid::from_u128(1)));

    assert!(ZeroableUlid::from_parts((1 << 48) - 1, (1 << 80) - 1).is_ok());

    assert_eq!(
        ZeroableUlid::from_parts((1 << 48) - 1, 1 << 80),
        Err(Error::RandomnessOutOfRange)
    );
    assert_eq!(
        ZeroableUlid::from_parts(1 << 48, (1 << 80) - 1),
        Err(Error::TimestampOutOfRange)
    );
}

#[test]
fn test_canonicalize() {
    let src = "0abcdefghijklmnopqrstvwxyz"; // cspell::disable-line
    let exp = "0ABCDEFGH1JK1MN0PQRSTVWXYZ"; // cspell::disable-line

    let r1 = canonicalize(src);
    assert!(r1.is_ok());

    let c1 = r1.unwrap();
    assert!(matches!(c1, Cow::Owned(_)));

    assert_eq!(c1, exp);

    let r2 = canonicalize(&c1);
    assert!(r2.is_ok());

    let c2 = r2.unwrap();
    assert!(matches!(c2, Cow::Borrowed(_)));

    assert_eq!(c2, exp);

    assert_eq!(
        canonicalize("000000000oooooooooOOOOOOOO"),
        Ok("00000000000000000000000000".into())
    );
    assert_eq!(
        canonicalize("iiiiiiiiiillllllllll111111"), // cspell::disable-line
        Ok("11111111111111111111111111".into())
    );
    assert_eq!(
        canonicalize("7zzzzzzzzzzzzzzzzzzzzzzzzz"),
        Ok("7ZZZZZZZZZZZZZZZZZZZZZZZZZ".into())
    );
    assert_eq!(canonicalize("80000000000000000000000000"), Err(Error::InvalidChar));
    assert_eq!(canonicalize("zzzzzzzzzzzzzzzzzzzzzzzzzz"), Err(Error::InvalidChar));

    assert_eq!(canonicalize(""), Err(Error::TooShort));

    assert_eq!(canonicalize("1234567890123456789012345"), Err(Error::TooShort));
    assert_eq!(canonicalize("123456789012345678901234567"), Err(Error::TooLong));
}

#[test]
fn test_validate() {
    // cspell::disable-next-line
    assert!(validate("0abcdefghijklmnopqrstvwxyz").is_ok());

    assert!(validate("oooooooooooooooooooooooooo").is_ok(),);
    assert!(validate("iiiiiiiiiiiiiiiiiiiiiiiiii").is_ok(),);
    assert!(validate("llllllllllllllllllllllllll").is_ok(),);
    assert!(validate("OOOOOOOOOOOOOOOOOOOOOOOOOO").is_ok(),);
    assert!(validate("IIIIIIIIIIIIIIIIIIIIIIIIII").is_ok(),);
    assert!(validate("LLLLLLLLLLLLLLLLLLLLLLLLLL").is_ok(),);

    assert!(validate("7zzzzzzzzzzzzzzzzzzzzzzzzz").is_ok(),);

    assert_eq!(validate("80000000000000000000000000"), Err(Error::InvalidChar));
    assert_eq!(validate("zzzzzzzzzzzzzzzzzzzzzzzzzz"), Err(Error::InvalidChar));

    assert_eq!(validate(""), Err(Error::TooShort));

    assert_eq!(validate("1234567890123456789012345"), Err(Error::TooShort));
    assert_eq!(validate("123456789012345678901234567"), Err(Error::TooLong));
}

// --- From/TryFrom trait tests for Ulid ---

#[test]
fn test_ulid_try_from_zeroable_ulid() {
    // Non-zero ZeroableUlid converts successfully
    let zu = ZeroableUlid::new();
    let u = Ulid::try_from(zu);
    assert!(u.is_ok());
    assert_eq!(u.unwrap().to_u128(), zu.to_u128());

    // Zero ZeroableUlid fails
    let zu_zero = ZeroableUlid::zeroed();
    assert_eq!(Ulid::try_from(zu_zero), Err(Error::InvalidZero));
}

#[test]
fn test_ulid_into_u128() {
    let u = Ulid::new();
    let n: u128 = u.into();
    assert_eq!(n, u.to_u128());
    assert_ne!(n, 0);
}

#[test]
fn test_ulid_try_from_u128() {
    // Non-zero succeeds
    let u = Ulid::try_from(42_u128);
    assert!(u.is_ok());
    assert_eq!(u.unwrap().to_u128(), 42);

    // Max value succeeds
    let u_max = Ulid::try_from(u128::MAX);
    assert!(u_max.is_ok());
    assert_eq!(u_max.unwrap().to_u128(), u128::MAX);

    // Zero fails
    assert_eq!(Ulid::try_from(0_u128), Err(Error::InvalidZero));
}

#[test]
fn test_ulid_into_nonzero_u128() {
    let u = Ulid::new();
    let nz: std::num::NonZero<u128> = u.into();
    assert_eq!(nz.get(), u.to_u128());
}

#[test]
fn test_ulid_from_nonzero_u128() {
    let nz = std::num::NonZero::new(99_u128).unwrap();
    let u = Ulid::from(nz);
    assert_eq!(u.to_u128(), 99);
}

#[test]
fn test_ulid_into_bytes() {
    let u = Ulid::new();
    let bytes: [u8; 16] = u.into();
    assert_eq!(bytes, u.to_bytes());
    assert_eq!(u128::from_be_bytes(bytes), u.to_u128());
}

#[test]
fn test_ulid_try_from_byte_array() {
    // Non-zero bytes succeed
    let bytes: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let u = Ulid::try_from(bytes);
    assert!(u.is_ok());
    assert_eq!(u.unwrap().to_u128(), 1);

    // Round-trip
    let original = Ulid::new();
    let bytes = original.to_bytes();
    assert_eq!(Ulid::try_from(bytes).unwrap(), original);

    // All-zero bytes fail
    let zero_bytes: [u8; 16] = [0; 16];
    assert_eq!(Ulid::try_from(zero_bytes), Err(Error::InvalidZero));
}

#[test]
fn test_ulid_try_from_byte_array_ref() {
    // Non-zero bytes succeed
    let bytes: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42];
    let u = Ulid::try_from(&bytes);
    assert!(u.is_ok());
    assert_eq!(u.unwrap().to_u128(), 42);

    // Round-trip
    let original = Ulid::new();
    let bytes = original.to_bytes();
    assert_eq!(Ulid::try_from(&bytes).unwrap(), original);

    // All-zero bytes fail
    let zero_bytes: [u8; 16] = [0; 16];
    assert_eq!(Ulid::try_from(&zero_bytes), Err(Error::InvalidZero));
}

#[test]
fn test_ulid_try_from_byte_slice() {
    // Correct length, non-zero
    let bytes: &[u8] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7];
    let u = Ulid::try_from(bytes);
    assert!(u.is_ok());
    assert_eq!(u.unwrap().to_u128(), 7);

    // Round-trip
    let original = Ulid::new();
    let bytes = original.to_bytes();
    assert_eq!(Ulid::try_from(bytes.as_slice()).unwrap(), original);

    // All-zero slice fails
    let zero_bytes: &[u8] = &[0; 16];
    assert_eq!(Ulid::try_from(zero_bytes), Err(Error::InvalidZero));

    // Too short
    let short: &[u8] = &[1, 2, 3];
    assert_eq!(Ulid::try_from(short), Err(Error::TooShort));

    // Too long
    let long: &[u8] = &[0; 17];
    assert_eq!(Ulid::try_from(long), Err(Error::TooLong));

    // Empty slice
    let empty: &[u8] = &[];
    assert_eq!(Ulid::try_from(empty), Err(Error::TooShort));
}

// --- From/TryFrom trait tests for ZeroableUlid ---

#[test]
fn test_zeroable_ulid_from_ulid() {
    let u = Ulid::new();
    let zu = ZeroableUlid::from(u);
    assert_eq!(zu.to_u128(), u.to_u128());
    assert!(!zu.is_zero());
}

#[test]
fn test_zeroable_ulid_into_u128() {
    let zu = ZeroableUlid::new();
    let n: u128 = zu.into();
    assert_eq!(n, zu.to_u128());
    assert_ne!(n, 0);

    // Zero case
    let zu_zero = ZeroableUlid::zeroed();
    let n_zero: u128 = zu_zero.into();
    assert_eq!(n_zero, 0);
}

#[test]
fn test_zeroable_ulid_from_u128() {
    let zu = ZeroableUlid::from(42_u128);
    assert_eq!(zu.to_u128(), 42);

    // Zero is valid
    let zu_zero = ZeroableUlid::from(0_u128);
    assert!(zu_zero.is_zero());

    // Max value
    let zu_max = ZeroableUlid::from(u128::MAX);
    assert_eq!(zu_max.to_u128(), u128::MAX);
}

#[test]
fn test_zeroable_ulid_into_bytes() {
    let zu = ZeroableUlid::new();
    let bytes: [u8; 16] = zu.into();
    assert_eq!(bytes, zu.to_bytes());
    assert_eq!(u128::from_be_bytes(bytes), zu.to_u128());

    // Zero case
    let zu_zero = ZeroableUlid::zeroed();
    let bytes_zero: [u8; 16] = zu_zero.into();
    assert_eq!(bytes_zero, [0; 16]);
}

#[test]
fn test_zeroable_ulid_from_byte_array() {
    let bytes: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let zu = ZeroableUlid::from(bytes);
    assert_eq!(zu.to_u128(), 1);

    // Round-trip
    let original = ZeroableUlid::new();
    let bytes = original.to_bytes();
    assert_eq!(ZeroableUlid::from(bytes), original);

    // All-zero is valid
    let zero_bytes: [u8; 16] = [0; 16];
    let zu_zero = ZeroableUlid::from(zero_bytes);
    assert!(zu_zero.is_zero());
}

#[test]
fn test_zeroable_ulid_from_byte_array_ref() {
    let bytes: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 42];
    let zu = ZeroableUlid::from(&bytes);
    assert_eq!(zu.to_u128(), 42);

    // Round-trip
    let original = ZeroableUlid::new();
    let bytes = original.to_bytes();
    assert_eq!(ZeroableUlid::from(&bytes), original);

    // All-zero is valid
    let zero_bytes: [u8; 16] = [0; 16];
    let zu_zero = ZeroableUlid::from(&zero_bytes);
    assert!(zu_zero.is_zero());
}

#[test]
fn test_zeroable_ulid_try_from_byte_slice() {
    // Correct length
    let bytes: &[u8] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7];
    let zu = ZeroableUlid::try_from(bytes);
    assert!(zu.is_ok());
    assert_eq!(zu.unwrap().to_u128(), 7);

    // Round-trip
    let original = ZeroableUlid::new();
    let bytes = original.to_bytes();
    assert_eq!(ZeroableUlid::try_from(bytes.as_slice()).unwrap(), original);

    // All-zero slice is valid for ZeroableUlid
    let zero_bytes: &[u8] = &[0; 16];
    let zu_zero = ZeroableUlid::try_from(zero_bytes);
    assert!(zu_zero.is_ok());
    assert!(zu_zero.unwrap().is_zero());

    // Too short
    let short: &[u8] = &[1, 2, 3];
    assert_eq!(ZeroableUlid::try_from(short), Err(Error::TooShort));

    // Too long
    let long: &[u8] = &[0; 17];
    assert_eq!(ZeroableUlid::try_from(long), Err(Error::TooLong));

    // Empty slice
    let empty: &[u8] = &[];
    assert_eq!(ZeroableUlid::try_from(empty), Err(Error::TooShort));
}

// --- Constants ---

#[test]
fn test_ulid_min_max() {
    assert_eq!(Ulid::MIN.to_u128(), 1);
    assert_eq!(Ulid::MAX.to_u128(), u128::MAX);
    assert!(Ulid::MIN < Ulid::MAX);

    // MIN is the smallest valid Ulid
    assert!(Ulid::from_u128(0).is_none());
    assert!(Ulid::from_u128(1).is_some());
    assert_eq!(Ulid::from_u128(1).unwrap(), Ulid::MIN);
    assert_eq!(Ulid::from_u128(u128::MAX).unwrap(), Ulid::MAX);
}

#[test]
fn test_zeroable_ulid_min_max() {
    assert_eq!(ZeroableUlid::MIN.to_u128(), 0);
    assert_eq!(ZeroableUlid::MAX.to_u128(), u128::MAX);

    assert_eq!(ZeroableUlid::from_u128(0), ZeroableUlid::MIN);
    assert_eq!(ZeroableUlid::from_u128(u128::MAX), ZeroableUlid::MAX);
}

// --- Default trait ---

#[test]
fn test_ulid_default() {
    let u1 = Ulid::default();
    let u2 = Ulid::default();

    // Default generates unique ULIDs (same as new())
    assert_ne!(u1, u2);
    assert!(u1 < u2);
    assert_ne!(u1.to_u128(), 0);
}

#[test]
fn test_zeroable_ulid_default() {
    let zu = ZeroableUlid::default();

    // Default for ZeroableUlid is zeroed
    assert!(zu.is_zero());
    assert_eq!(zu.to_u128(), 0);
    assert_eq!(zu, ZeroableUlid::zeroed());
}

// --- try_new ---

#[test]
fn test_ulid_try_new() {
    let u1 = Ulid::try_new();
    let u2 = Ulid::try_new();
    assert!(u1.is_some());
    assert!(u2.is_some());

    let u1 = u1.unwrap();
    let u2 = u2.unwrap();
    assert_ne!(u1, u2);
    assert!(u1 < u2);
}

#[test]
fn test_zeroable_ulid_try_new() {
    let zu1 = ZeroableUlid::try_new();
    let zu2 = ZeroableUlid::try_new();
    assert!(zu1.is_some());
    assert!(zu2.is_some());

    let zu1 = zu1.unwrap();
    let zu2 = zu2.unwrap();
    assert_ne!(zu1, zu2);
    assert!(zu1 < zu2);
    assert!(!zu1.is_zero());
    assert!(!zu2.is_zero());
}

// --- Known-value decomposition ---

#[test]
fn test_ulid_parts_known_values() {
    // Create from known parts, verify decomposition
    let u = Ulid::from_parts(1000, 42).unwrap();
    assert_eq!(u.timestamp(), 1000);
    assert_eq!(u.randomness(), 42);
    assert_eq!(u.to_parts(), (1000, 42));

    // Round-trip from_parts → to_parts
    let (ts, rnd) = u.to_parts();
    let u2 = Ulid::from_parts(ts, rnd).unwrap();
    assert_eq!(u, u2);

    // Boundary: max timestamp, max randomness
    let u_max_parts = Ulid::from_parts((1 << 48) - 1, (1 << 80) - 1).unwrap();
    assert_eq!(u_max_parts.timestamp(), (1 << 48) - 1);
    assert_eq!(u_max_parts.randomness(), (1 << 80) - 1);

    // Timestamp only (randomness = 0 is fine for Ulid when timestamp > 0)
    let u_ts_only = Ulid::from_parts(1, 0).unwrap();
    assert_eq!(u_ts_only.timestamp(), 1);
    assert_eq!(u_ts_only.randomness(), 0);

    // Randomness only (timestamp = 0 is fine for Ulid when randomness > 0)
    let u_rnd_only = Ulid::from_parts(0, 1).unwrap();
    assert_eq!(u_rnd_only.timestamp(), 0);
    assert_eq!(u_rnd_only.randomness(), 1);

    // Both zero → error for Ulid
    assert_eq!(Ulid::from_parts(0, 0), Err(Error::InvalidZero));

    // Out-of-range errors
    assert_eq!(Ulid::from_parts(1 << 48, 0), Err(Error::TimestampOutOfRange));
    assert_eq!(Ulid::from_parts(0, 1 << 80), Err(Error::RandomnessOutOfRange));
}

#[test]
fn test_zeroable_ulid_parts_known_values() {
    // Create from known parts
    let zu = ZeroableUlid::from_parts(5000, 99).unwrap();
    assert_eq!(zu.timestamp(), 5000);
    assert_eq!(zu.randomness(), 99);
    assert_eq!(zu.to_parts(), (5000, 99));

    // Round-trip
    let (ts, rnd) = zu.to_parts();
    let zu2 = ZeroableUlid::from_parts(ts, rnd).unwrap();
    assert_eq!(zu, zu2);

    // Zero is valid for ZeroableUlid
    let zu_zero = ZeroableUlid::from_parts(0, 0).unwrap();
    assert!(zu_zero.is_zero());
    assert_eq!(zu_zero.timestamp(), 0);
    assert_eq!(zu_zero.randomness(), 0);

    // Boundary: max values
    let zu_max = ZeroableUlid::from_parts((1 << 48) - 1, (1 << 80) - 1).unwrap();
    assert_eq!(zu_max.timestamp(), (1 << 48) - 1);
    assert_eq!(zu_max.randomness(), (1 << 80) - 1);

    // Out-of-range errors
    assert_eq!(ZeroableUlid::from_parts(1 << 48, 0), Err(Error::TimestampOutOfRange));
    assert_eq!(ZeroableUlid::from_parts(0, 1 << 80), Err(Error::RandomnessOutOfRange));
}

// --- Parse and Display ---

#[test]
fn test_ulid_parse_and_display() {
    // Round-trip: generate → display → parse
    let u1 = Ulid::new();
    let s = u1.to_string();
    assert_eq!(s.len(), 26);
    let u2: Ulid = s.parse().unwrap();
    assert_eq!(u1, u2);

    // Case insensitive round-trip
    let u3: Ulid = s.to_lowercase().parse().unwrap();
    assert_eq!(u1, u3);

    // Disambiguation: i/l → 1, o → 0
    // cspell:disable
    let a: Ulid = "01JAVEE2CB2R1MP14KP0AW1W1Z".parse().unwrap();
    let b: Ulid = "01javee2cb2r1mp14kp0aw1w1z".parse().unwrap();
    let c: Ulid = "0ijavee2cb2rlmpl4kpoawlwlz".parse().unwrap();
    // cspell:enable
    assert_eq!(a, b);
    assert_eq!(a, c);

    // Zero string → error for Ulid
    assert_eq!("00000000000000000000000000".parse::<Ulid>(), Err(Error::InvalidZero));

    // Invalid characters
    assert_eq!("0000000000000000000000000U".parse::<Ulid>(), Err(Error::InvalidChar));
    assert_eq!("0000000000000000000000000u".parse::<Ulid>(), Err(Error::InvalidChar));
    assert_eq!("000000000000000000000000$$".parse::<Ulid>(), Err(Error::InvalidChar));

    // Overflow (first char > 7)
    assert_eq!("80000000000000000000000000".parse::<Ulid>(), Err(Error::InvalidChar));

    // Wrong length
    assert_eq!("".parse::<Ulid>(), Err(Error::TooShort));
    assert_eq!("0".parse::<Ulid>(), Err(Error::TooShort));
    assert_eq!("0000000000000000000000000".parse::<Ulid>(), Err(Error::TooShort));
    assert_eq!("000000000000000000000000000".parse::<Ulid>(), Err(Error::TooLong));
}

#[test]
fn test_zeroable_ulid_display() {
    // Generated ULID displays as 26 chars
    let zu = ZeroableUlid::new();
    let s = zu.to_string();
    assert_eq!(s.len(), 26);

    // Zeroed displays as all zeros
    let zu_zero = ZeroableUlid::zeroed();
    assert_eq!(zu_zero.to_string(), "00000000000000000000000000");

    // MAX displays as max string
    assert_eq!(ZeroableUlid::MAX.to_string(), "7ZZZZZZZZZZZZZZZZZZZZZZZZZ");

    // Display round-trip
    let zu2: ZeroableUlid = s.parse().unwrap();
    assert_eq!(zu, zu2);
}

#[test]
fn test_zeroable_ulid_string_length() {
    assert_eq!(ZeroableUlid::new().to_string().len(), 26);
    assert_eq!(ZeroableUlid::zeroed().to_string().len(), 26);
}

// --- Bytes round-trip ---

#[test]
fn test_ulid_bytes_known_values() {
    // Create from known parts, verify byte representation (big-endian)
    let u = Ulid::from_parts(1, 1).unwrap();
    let bytes = u.to_bytes();

    // timestamp 1 in top 48 bits, randomness 1 in bottom 80 bits
    // u128 = (1 << 80) | 1
    let expected_u128: u128 = (1_u128 << 80) | 1;
    assert_eq!(u128::from_be_bytes(bytes), expected_u128);

    // Round-trip
    assert_eq!(Ulid::from_bytes(bytes).unwrap(), u);

    // from_bytes with all zeros → None
    assert!(Ulid::from_bytes([0; 16]).is_none());

    // from_bytes with single non-zero byte
    let mut bytes_one = [0u8; 16];
    bytes_one[15] = 1;
    let u_one = Ulid::from_bytes(bytes_one).unwrap();
    assert_eq!(u_one.to_u128(), 1);
}

#[test]
fn test_zeroable_ulid_bytes_known_values() {
    // Zero bytes
    let zu_zero = ZeroableUlid::from_bytes([0; 16]);
    assert!(zu_zero.is_zero());
    assert_eq!(zu_zero.to_bytes(), [0; 16]);

    // Known value
    let zu = ZeroableUlid::from_parts(1, 1).unwrap();
    let bytes = zu.to_bytes();
    let zu2 = ZeroableUlid::from_bytes(bytes);
    assert_eq!(zu, zu2);

    // Max value
    let zu_max = ZeroableUlid::from_bytes([0xFF; 16]);
    assert_eq!(zu_max.to_u128(), u128::MAX);
}

// --- Cross-type conversions ---

#[test]
fn test_ulid_zeroable_conversions() {
    let u = Ulid::new();

    // Ulid → ZeroableUlid (via method)
    let zu = u.to_zeroable_ulid();
    assert_eq!(u.to_u128(), zu.to_u128());
    assert!(!zu.is_zero());

    // ZeroableUlid → Ulid (via method)
    let u2 = Ulid::from_zeroable_ulid(zu).unwrap();
    assert_eq!(u, u2);

    // ZeroableUlid zero → Ulid fails
    let zu_zero = ZeroableUlid::zeroed();
    assert!(Ulid::from_zeroable_ulid(zu_zero).is_none());
}

#[test]
fn test_zeroable_ulid_to_from_ulid() {
    let u = Ulid::new();

    // from_ulid
    let zu = ZeroableUlid::from_ulid(u);
    assert_eq!(zu.to_u128(), u.to_u128());

    // to_ulid for non-zero
    let u2 = zu.to_ulid();
    assert!(u2.is_some());
    assert_eq!(u2.unwrap(), u);

    // to_ulid for zero
    let zu_zero = ZeroableUlid::zeroed();
    assert!(zu_zero.to_ulid().is_none());
}

// --- Datetime ---

#[test]
#[cfg_attr(miri, ignore)]
fn test_ulid_datetime() {
    let u = Ulid::new();

    // datetime should be close to now
    let dt = u.datetime();
    let now = SystemTime::now();
    let diff = now.duration_since(dt).unwrap();
    assert!(diff.as_secs() < 1);

    // try_datetime should return the same thing
    let try_dt = u.try_datetime();
    assert_eq!(try_dt, Some(dt));

    // Known timestamp: create from parts with timestamp = 0 → epoch
    let u_epoch = Ulid::from_parts(0, 1).unwrap();
    assert_eq!(u_epoch.datetime(), SystemTime::UNIX_EPOCH);
    assert_eq!(u_epoch.try_datetime(), Some(SystemTime::UNIX_EPOCH));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_zeroable_ulid_datetime() {
    let zu = ZeroableUlid::new();

    let dt = zu.datetime();
    let now = SystemTime::now();
    let diff = now.duration_since(dt).unwrap();
    assert!(diff.as_secs() < 1);

    assert_eq!(zu.try_datetime(), Some(dt));

    // Zero → epoch
    let zu_zero = ZeroableUlid::zeroed();
    assert_eq!(zu_zero.datetime(), SystemTime::UNIX_EPOCH);
}

// --- Ordering ---

#[test]
fn test_ulid_ordering() {
    // Known values: smaller timestamp < larger timestamp
    let u_small = Ulid::from_parts(100, 50).unwrap();
    let u_large = Ulid::from_parts(200, 25).unwrap();
    assert!(u_small < u_large);

    // Same timestamp: smaller randomness < larger randomness
    let u_a = Ulid::from_parts(100, 1).unwrap();
    let u_b = Ulid::from_parts(100, 2).unwrap();
    assert!(u_a < u_b);

    // Eq
    let u_c = Ulid::from_parts(100, 1).unwrap();
    assert_eq!(u_a, u_c);

    // MIN < any generated ULID
    assert!(Ulid::MIN < Ulid::new());

    // any generated ULID < MAX (effectively always true since generation is bounded)
    assert!(Ulid::new() < Ulid::MAX);
}

#[test]
fn test_zeroable_ulid_ordering() {
    let zu_zero = ZeroableUlid::zeroed();
    let zu_nonzero = ZeroableUlid::new();

    // Zero < any non-zero
    assert!(zu_zero < zu_nonzero);

    // MIN == zeroed
    assert_eq!(ZeroableUlid::MIN, zu_zero);
}

// --- Hash ---

#[test]
fn test_ulid_hash() {
    fn hash_of<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    // Equal ULIDs have equal hashes
    let u1 = Ulid::from_u128(42).unwrap();
    let u2 = Ulid::from_u128(42).unwrap();
    assert_eq!(hash_of(&u1), hash_of(&u2));

    // Different ULIDs have different hashes (probabilistically)
    let u3 = Ulid::from_u128(43).unwrap();
    assert_ne!(hash_of(&u1), hash_of(&u3));
}

#[test]
fn test_zeroable_ulid_hash() {
    fn hash_of<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    let zu1 = ZeroableUlid::from_u128(42);
    let zu2 = ZeroableUlid::from_u128(42);
    assert_eq!(hash_of(&zu1), hash_of(&zu2));

    let zu3 = ZeroableUlid::from_u128(43);
    assert_ne!(hash_of(&zu1), hash_of(&zu3));

    // Zeroed has a consistent hash
    let zu_z1 = ZeroableUlid::zeroed();
    let zu_z2 = ZeroableUlid::zeroed();
    assert_eq!(hash_of(&zu_z1), hash_of(&zu_z2));
}

// --- Clone / Copy ---

#[test]
fn test_ulid_clone_copy() {
    let u1 = Ulid::new();

    // Copy
    let u2 = u1;
    assert_eq!(u1, u2); // u1 still usable (Copy)

    // Clone
    #[allow(clippy::clone_on_copy)]
    let u3 = u1.clone();
    assert_eq!(u1, u3);
}

#[test]
fn test_zeroable_ulid_clone_copy() {
    let zu1 = ZeroableUlid::new();

    let zu2 = zu1;
    assert_eq!(zu1, zu2);

    #[allow(clippy::clone_on_copy)]
    let zu3 = zu1.clone();
    assert_eq!(zu1, zu3);

    // Also for zeroed
    let zu_z = ZeroableUlid::zeroed();
    let zu_z2 = zu_z;
    assert_eq!(zu_z, zu_z2);
}

// --- ZeroableUlid monotonicity and uniqueness ---

#[test]
fn test_zeroable_ulid_monotonicity() {
    let zu1 = ZeroableUlid::new();
    let zu2 = ZeroableUlid::new();
    let zu3 = ZeroableUlid::new();

    assert!(zu1 < zu2);
    assert!(zu2 < zu3);
}

#[test]
fn test_zeroable_ulid_uniques() {
    let zu1 = ZeroableUlid::new();
    let zu2 = ZeroableUlid::new();
    let zu3 = ZeroableUlid::new();

    assert_ne!(zu1, zu2);
    assert_ne!(zu2, zu3);
    assert_ne!(zu3, zu1);
}

// --- Error type ---

#[test]
fn test_error_debug() {
    // Debug should be derivable and not panic
    let s = format!("{:?}", Error::TooShort);
    assert!(!s.is_empty());
    assert!(s.contains("TooShort"));
}

#[test]
fn test_error_std_error() {
    // Error implements std::error::Error
    fn accepts_std_error(_: &dyn std::error::Error) {}

    let err = Error::InvalidChar;
    accepts_std_error(&err);

    // source() should return None (no underlying cause)
    assert!(std::error::Error::source(&err).is_none());
}

#[test]
fn test_error_clone_copy() {
    let e1 = Error::InvalidZero;

    // Copy
    let e2 = e1;
    assert_eq!(e1, e2);

    // Clone
    #[allow(clippy::clone_on_copy)]
    let e3 = e1.clone();
    assert_eq!(e1, e3);
}

#[test]
fn test_error_hash() {
    fn hash_of<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    // Same variant → same hash
    assert_eq!(hash_of(&Error::TooShort), hash_of(&Error::TooShort));

    // Different variants → different hash (probabilistically)
    assert_ne!(hash_of(&Error::TooShort), hash_of(&Error::TooLong));
}

#[test]
fn test_error_eq() {
    assert_eq!(Error::TooShort, Error::TooShort);
    assert_ne!(Error::TooShort, Error::TooLong);
    assert_ne!(Error::InvalidChar, Error::InvalidZero);
    assert_ne!(Error::TimestampOutOfRange, Error::RandomnessOutOfRange);
}

// --- Serde ---

#[cfg(feature = "serde")]
mod serde_tests {
    use crate::*;

    #[test]
    fn test_serde_ulid_roundtrip() {
        let u1 = Ulid::new();
        let json = serde_json::to_string(&u1).unwrap();

        // Serialized as a 26-char string in quotes
        assert_eq!(json.len(), 28); // 26 chars + 2 quotes
        assert!(json.starts_with('"'));
        assert!(json.ends_with('"'));

        // Deserialize back
        let u2: Ulid = serde_json::from_str(&json).unwrap();
        assert_eq!(u1, u2);

        // Serialized string matches Display
        let display = u1.to_string();
        assert_eq!(json, format!("\"{display}\""));
    }

    #[test]
    fn test_serde_zeroable_ulid_roundtrip() {
        let zu1 = ZeroableUlid::new();
        let json = serde_json::to_string(&zu1).unwrap();

        let zu2: ZeroableUlid = serde_json::from_str(&json).unwrap();
        assert_eq!(zu1, zu2);
    }

    #[test]
    fn test_serde_zeroable_ulid_zero() {
        let zu = ZeroableUlid::zeroed();
        let json = serde_json::to_string(&zu).unwrap();
        assert_eq!(json, "\"00000000000000000000000000\"");

        let zu2: ZeroableUlid = serde_json::from_str(&json).unwrap();
        assert!(zu2.is_zero());
    }

    #[test]
    fn test_serde_ulid_zero_string_error() {
        // Deserializing a zero ULID string as Ulid should fail
        let json = "\"00000000000000000000000000\"";
        let result = serde_json::from_str::<Ulid>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_serde_invalid_string() {
        // Too short
        let result = serde_json::from_str::<Ulid>("\"ABC\"");
        assert!(result.is_err());

        // Invalid character
        let result = serde_json::from_str::<Ulid>("\"0000000000000000000000000U\"");
        assert!(result.is_err());

        // Not a string at all
        let result = serde_json::from_str::<Ulid>("42");
        assert!(result.is_err());

        // Same for ZeroableUlid
        let result = serde_json::from_str::<ZeroableUlid>("\"ABC\"");
        assert!(result.is_err());

        let result = serde_json::from_str::<ZeroableUlid>("42");
        assert!(result.is_err());
    }

    #[test]
    fn test_serde_ulid_in_struct() {
        use serde_derive::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Item {
            id: Ulid,
            name: String,
        }

        let item1 = Item {
            id: Ulid::new(),
            name: "test".into(),
        };

        let json = serde_json::to_string(&item1).unwrap();
        let item2: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item1, item2);
    }

    #[test]
    fn test_serde_zeroable_ulid_in_struct() {
        use serde_derive::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct Item {
            id: ZeroableUlid,
        }

        // Non-zero
        let item1 = Item {
            id: ZeroableUlid::new(),
        };
        let json = serde_json::to_string(&item1).unwrap();
        let item2: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item1, item2);

        // Zero
        let item_zero = Item {
            id: ZeroableUlid::zeroed(),
        };
        let json = serde_json::to_string(&item_zero).unwrap();
        let item_zero2: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item_zero, item_zero2);
    }

    #[test]
    fn test_serde_max_value() {
        let u = Ulid::MAX;
        let json = serde_json::to_string(&u).unwrap();
        assert_eq!(json, "\"7ZZZZZZZZZZZZZZZZZZZZZZZZZ\"");
        let u2: Ulid = serde_json::from_str(&json).unwrap();
        assert_eq!(u, u2);

        let zu = ZeroableUlid::MAX;
        let json = serde_json::to_string(&zu).unwrap();
        assert_eq!(json, "\"7ZZZZZZZZZZZZZZZZZZZZZZZZZ\"");
        let zu2: ZeroableUlid = serde_json::from_str(&json).unwrap();
        assert_eq!(zu, zu2);
    }

    #[test]
    fn test_serde_case_insensitive_deserialize() {
        // Lowercase should deserialize correctly
        // cspell:disable
        let json = "\"01javee2cb2r1mp14kp0aw1w1z\"";
        // cspell:enable
        let u: Ulid = serde_json::from_str(json).unwrap();
        let zu: ZeroableUlid = serde_json::from_str(json).unwrap();
        assert_eq!(u.to_u128(), zu.to_u128());
    }
}

// --- Unsafe unchecked constructors ---

#[test]
fn test_ulid_from_u128_unchecked() {
    let u = unsafe { Ulid::from_u128_unchecked(42) };
    assert_eq!(u.to_u128(), 42);

    let u_max = unsafe { Ulid::from_u128_unchecked(u128::MAX) };
    assert_eq!(u_max, Ulid::MAX);
}

#[test]
fn test_ulid_from_bytes_unchecked() {
    let bytes: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1];
    let u = unsafe { Ulid::from_bytes_unchecked(bytes) };
    assert_eq!(u.to_u128(), 1);
    assert_eq!(u.to_bytes(), bytes);
}

#[test]
fn test_ulid_from_parts_unchecked() {
    let u = unsafe { Ulid::from_parts_unchecked(1000, 42) };
    assert_eq!(u.timestamp(), 1000);
    assert_eq!(u.randomness(), 42);

    // Should equal the safe version
    let u_safe = Ulid::from_parts(1000, 42).unwrap();
    assert_eq!(u, u_safe);
}

#[test]
fn test_zeroable_ulid_from_parts_unchecked() {
    let zu = unsafe { ZeroableUlid::from_parts_unchecked(1000, 42) };
    assert_eq!(zu.timestamp(), 1000);
    assert_eq!(zu.randomness(), 42);

    // Should equal the safe version
    let zu_safe = ZeroableUlid::from_parts(1000, 42).unwrap();
    assert_eq!(zu, zu_safe);
}

// --- Display consistency between types ---

#[test]
fn test_display_consistency_between_types() {
    // The same u128 value should produce the same string for both types
    let u = Ulid::new();
    let zu = u.to_zeroable_ulid();
    assert_eq!(u.to_string(), zu.to_string());

    // try_to_string should match to_string
    assert_eq!(u.try_to_string().unwrap(), u.to_string());
    assert_eq!(zu.try_to_string().unwrap(), zu.to_string());
}

// --- try_to_string for ZeroableUlid zeroed ---

#[test]
fn test_try_to_string_zeroed() {
    let zu = ZeroableUlid::zeroed();
    let s = zu.try_to_string();
    assert!(s.is_some());
    assert_eq!(s.unwrap(), "00000000000000000000000000");
}

// --- is_zero ---

#[test]
fn test_zeroable_ulid_is_zero() {
    assert!(ZeroableUlid::zeroed().is_zero());
    assert!(ZeroableUlid::from_u128(0).is_zero());
    assert!(ZeroableUlid::MIN.is_zero());

    assert!(!ZeroableUlid::new().is_zero());
    assert!(!ZeroableUlid::from_u128(1).is_zero());
    assert!(!ZeroableUlid::MAX.is_zero());
}

// --- Ulid::from_u128 edge cases ---

#[test]
fn test_ulid_from_u128_edge_cases() {
    // 0 → None
    assert!(Ulid::from_u128(0).is_none());

    // 1 → Some (MIN)
    assert_eq!(Ulid::from_u128(1), Some(Ulid::MIN));

    // u128::MAX → Some (MAX)
    assert_eq!(Ulid::from_u128(u128::MAX), Some(Ulid::MAX));

    // Round-trip for any value
    let u = Ulid::from_u128(123_456_789).unwrap();
    assert_eq!(u.to_u128(), 123_456_789);
}
