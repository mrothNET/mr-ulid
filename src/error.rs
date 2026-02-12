use std::fmt;

/// Errors that can occur when creating ULIDs out of foreign data.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Error {
    /// The ULID string is too short.
    TooShort,
    /// The ULID string is too long.
    TooLong,
    /// The ULID string contains an invalid character.
    InvalidChar,
    /// The value for the ULID is zero.
    InvalidZero,
    /// The given timestamp for the ULID is too large.
    TimestampOutOfRange,
    /// The given randomness for the ULID is too large.
    RandomnessOutOfRange,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    /// Formats the error message for display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match *self {
            Self::TooShort => "string is too short",
            Self::TooLong => "string is too long",
            Self::InvalidChar => "string contains an invalid character",
            Self::InvalidZero => "invalid zero value",
            Self::TimestampOutOfRange => "timestamp is too large",
            Self::RandomnessOutOfRange => "randomness is too large",
        };
        write!(f, "{message}")
    }
}
