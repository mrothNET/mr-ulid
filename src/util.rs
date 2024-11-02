use std::fmt::Formatter;

use crate::{base32, Error, RANDOM_BITS, RANDOM_MASK};

pub fn as_array<const N: usize>(bytes: &[u8]) -> Result<&[u8; N], Error> {
    use std::cmp::Ordering;

    match bytes.len().cmp(&26) {
        Ordering::Equal => Ok(bytes.try_into().unwrap()),
        Ordering::Less => Err(Error::ToShort),
        Ordering::Greater => Err(Error::ToLong),
    }
}

pub const fn from_parts(timestamp: u64, randomness: u128) -> Result<u128, Error> {
    match (timestamp as u128).checked_mul(1 << RANDOM_BITS) {
        None => Err(Error::TimestampOutOfRange),

        Some(shifted_timestamp) => {
            if randomness > RANDOM_MASK {
                Err(Error::RandomnessOutOfRange)
            } else {
                Ok(shifted_timestamp | randomness)
            }
        }
    }
}

pub fn try_to_string(ulid: u128) -> Option<String> {
    let mut s = String::new();
    s.try_reserve_exact(26).ok()?;

    let mut buffer = [0; 26];
    s.push_str(base32::encode(ulid, &mut buffer));

    Some(s)
}

pub fn debug_ulid(name: &str, ulid: u128, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
    struct Timestamp(u64);
    impl std::fmt::Debug for Timestamp {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            write!(f, "\"{ts}\"", ts = timestamp_to_string(self.0))
        }
    }

    struct Randomness(u128);
    impl std::fmt::Debug for Randomness {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            write!(f, "\"{:010X}\"", self.0)
        }
    }

    let mut buffer = [0; 26];

    let string = base32::encode(ulid, &mut buffer);
    let timestamp = Timestamp((ulid >> RANDOM_BITS) as u64);
    let randomness = Randomness(ulid & RANDOM_MASK);

    f.debug_struct(name)
        .field("string", &string)
        .field("timestamp", &timestamp)
        .field("randomness", &randomness)
        .finish()
}

fn timestamp_to_string(millis: u64) -> String {
    const DAYS_PER_YEAR: u64 = 365;
    const DAYS_PER_LEAP_YEAR: u64 = DAYS_PER_YEAR + 1;

    const DAYS_PER_QUAD_YEAR: u64 = 4 * DAYS_PER_YEAR + 1; // leap year: every 4 years,
    const DAYS_PER_CENTURY: u64 = 25 * DAYS_PER_QUAD_YEAR - 1; // but not every 100 years,
    const DASS_PER_QUADRICENTENNIAL: u64 = 4 * DAYS_PER_CENTURY + 1; // but again every 400 years.

    const BASE: u64 = 1600;
    const DAYS_BASE_TO_1970: u64 = 3 * DAYS_PER_CENTURY + 1 + 70 * DAYS_PER_YEAR + 70 / 4;

    let (seconds, millis) = (millis / 1000, (millis % 1000) as u32);
    let (minutes, seconds) = (seconds / 60, (seconds % 60) as u32);
    let (hours, minutes) = (minutes / 60, (minutes % 60) as u32);
    let (days, hours) = (hours / 24, (hours % 24) as u32);

    // days relative to year 1600
    let days = days + DAYS_BASE_TO_1970;

    let (quadricentennials, days) = (days / DASS_PER_QUADRICENTENNIAL, days % DASS_PER_QUADRICENTENNIAL);
    let (centuries, days) = (days / DAYS_PER_CENTURY, days % DAYS_PER_CENTURY);
    let (quad_years, days) = (days / DAYS_PER_QUAD_YEAR, days % DAYS_PER_QUAD_YEAR);

    let is_leap_year = days < DAYS_PER_LEAP_YEAR;

    let (years, days) = if is_leap_year {
        (0, days)
    } else {
        let days = days - DAYS_PER_LEAP_YEAR;
        let (normal_years, days) = (days / DAYS_PER_YEAR, days % DAYS_PER_YEAR);
        (normal_years + 1, days)
    };

    let year = BASE + quadricentennials * 400 + centuries * 100 + quad_years * 4 + years;

    #[rustfmt::skip]
    let days_in_month = [
        31,
        if is_leap_year { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];

    let mut days = days;
    let mut month = 0;
    while days >= days_in_month[month] {
        days -= days_in_month[month];
        month += 1;
    }

    let month = month + 1;
    let day = days + 1;

    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_to_string() {
        assert_eq!(timestamp_to_string(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(timestamp_to_string((1 << 48) - 1), "10889-08-02T05:31:50.655Z");
        assert_eq!(timestamp_to_string(327_403_382_400_000), "12345-01-01T00:00:00.000Z");

        assert_eq!(timestamp_to_string(1_709_164_799_999), "2024-02-28T23:59:59.999Z");
        assert_eq!(timestamp_to_string(1_709_164_800_000), "2024-02-29T00:00:00.000Z");

        assert_eq!(timestamp_to_string(1_740_787_199_999), "2025-02-28T23:59:59.999Z");
        assert_eq!(timestamp_to_string(1_740_787_200_000), "2025-03-01T00:00:00.000Z");

        assert_eq!(timestamp_to_string(1_735_689_599_000), "2024-12-31T23:59:59.000Z");
        assert_eq!(timestamp_to_string(1_735_689_599_999), "2024-12-31T23:59:59.999Z");
        assert_eq!(timestamp_to_string(1_735_689_600_000), "2025-01-01T00:00:00.000Z");
    }
}
