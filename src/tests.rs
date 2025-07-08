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

    assert_eq!("".parse::<ZeroableUlid>(), Err(Error::ToShort));

    assert_eq!("1234567890123456789012345".parse::<ZeroableUlid>(), Err(Error::ToShort));
    assert_eq!(
        "123456789012345678901234567".parse::<ZeroableUlid>(),
        Err(Error::ToLong)
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

    assert_eq!(canonicalize(""), Err(Error::ToShort));

    assert_eq!(canonicalize("1234567890123456789012345"), Err(Error::ToShort));
    assert_eq!(canonicalize("123456789012345678901234567"), Err(Error::ToLong));
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

    assert_eq!(validate(""), Err(Error::ToShort));

    assert_eq!(validate("1234567890123456789012345"), Err(Error::ToShort));
    assert_eq!(validate("123456789012345678901234567"), Err(Error::ToLong));
}
