use std::str::from_utf8_unchecked;

use crate::Error;

pub fn encode(mut n: u128, buffer: &mut [u8; 26]) -> &str {
    // cspell:disable-next-line
    const ALPHABET: [u8; 32] = *b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

    for byte in buffer.iter_mut().rev() {
        *byte = ALPHABET[(n & 0x1F) as usize];
        n >>= 5;
    }

    // Safety: Encoding above guarantees valid UTF-8
    unsafe { from_utf8_unchecked(buffer) }
}

pub fn decode(ascii_bytes: &[u8; 26]) -> Result<u128, Error> {
    #[rustfmt::skip]
    const DECODE: [i8; 256] = [
        /* 0x00 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0x10 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0x20 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0x30 */   0,  1,  2,  3,  4,  5,  6,  7,  8,  9, -1, -1, -1, -1, -1, -1,
        /* 0x40 */  -1, 10, 11, 12, 13, 14, 15, 16, 17,  1, 18, 19,  1, 20, 21,  0,
        /* 0x50 */  22, 23, 24, 25, 26, -1, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1,
        /* 0x60 */  -1, 10, 11, 12, 13, 14, 15, 16, 17,  1, 18, 19,  1, 20, 21,  0,
        /* 0x70 */  22, 23, 24, 25, 26, -1, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1,
        /* 0x80 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0x90 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0xA0 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0xB0 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0xC0 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0xD0 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0xE0 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        /* 0xF0 */  -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    ];

    fn decode(char: u8) -> Result<u128, Error> {
        u128::try_from(DECODE[usize::from(char)]).or(Err(Error::InvalidChar))
    }

    let mut n = decode(ascii_bytes[0])?;

    if n <= 7 {
        for &byte in &ascii_bytes[1..26] {
            n = (n << 5) | decode(byte)?;
        }
        Ok(n)
    } else {
        Err(Error::InvalidChar)
    }
}

pub fn validate(buffer: &[u8; 26]) -> Result<(), Error> {
    let first_ok = is_valid_first_char(buffer[0]);
    let rest_ok = buffer[1..].iter().all(|&c| is_valid_char(c));

    if first_ok && rest_ok {
        Ok(())
    } else {
        Err(Error::InvalidChar)
    }
}

pub fn canonicalize(buffer: &mut [u8; 26]) -> Result<&str, Error> {
    buffer[0] = normalize_first_char(buffer[0])?;

    for byte in &mut buffer[1..] {
        *byte = normalize_char(*byte)?;
    }

    // Safety: Above code guarantees valid UTF-8 (it returns early, when not)
    Ok(unsafe { from_utf8_unchecked(buffer) })
}

const fn is_valid_first_char(c: u8) -> bool {
    matches!(c, b'0'..=b'7' | b'o' | b'i' | b'l' | b'O' | b'I' | b'L')
}

const fn is_valid_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() && c != b'u' && c != b'U'
}

const fn normalize_first_char(c: u8) -> Result<u8, Error> {
    match c {
        b'0'..=b'7' => Ok(c),
        b'i' | b'I' | b'l' | b'L' => Ok(b'1'),
        b'o' | b'O' => Ok(b'0'),
        _ => Err(Error::InvalidChar),
    }
}

const fn normalize_char(c: u8) -> Result<u8, Error> {
    match c {
        b'i' | b'I' | b'l' | b'L' => Ok(b'1'),
        b'o' | b'O' => Ok(b'0'),
        b'u' | b'U' => Err(Error::InvalidChar),
        other if other.is_ascii_alphanumeric() => Ok(other.to_ascii_uppercase()),
        _ => Err(Error::InvalidChar),
    }
}
