pub const SCALE_1E8: i64 = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecimalParseError {
    Empty,
    InvalidChar,
    TooManyDecimals,
    Overflow,
}

pub fn parse_decimal_1e8(value: &str) -> Result<i64, DecimalParseError> {
    const POWERS_OF_TEN: [i64; 9] = [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000, SCALE_1E8];

    if value.len() > 18 {
        return parse_decimal_checked(value);
    }
    if value.is_empty() {
        return Err(DecimalParseError::Empty);
    }

    let mut raw = 0_i64;
    let mut scale = 8;
    for (index, byte) in value.bytes().enumerate() {
        let digit = byte.wrapping_sub(b'0');
        if digit <= 9 {
            raw = raw * 10 + i64::from(digit);
        } else if byte == b'.' && index != 0 && scale == 8 {
            let fractional_digits = value.len() - index - 1;
            if !(1..=8).contains(&fractional_digits) {
                return parse_decimal_checked(value);
            }
            scale -= fractional_digits;
        } else {
            return parse_decimal_checked(value);
        }
    }

    raw.checked_mul(POWERS_OF_TEN[scale])
        .ok_or(DecimalParseError::Overflow)
}

fn parse_decimal_checked(value: &str) -> Result<i64, DecimalParseError> {
    if value.is_empty() {
        return Err(DecimalParseError::Empty);
    }

    let mut raw = 0_i64;
    let mut integer_seen = false;
    let mut seen_dot = false;
    let mut fractional_digits = 0_u8;

    for byte in value.bytes() {
        match byte {
            b'0'..=b'9' => {
                if seen_dot {
                    if fractional_digits == 8 {
                        return Err(DecimalParseError::TooManyDecimals);
                    }
                    fractional_digits += 1;
                } else {
                    integer_seen = true;
                }

                raw = raw
                    .checked_mul(10)
                    .and_then(|raw| raw.checked_add(i64::from(byte - b'0')))
                    .ok_or(DecimalParseError::Overflow)?;
            }
            b'.' if integer_seen && !seen_dot => seen_dot = true,
            _invalid_byte => return Err(DecimalParseError::InvalidChar),
        }
    }

    if !integer_seen || (seen_dot && fractional_digits == 0) {
        return Err(DecimalParseError::InvalidChar);
    }

    while fractional_digits < 8 {
        raw = raw.checked_mul(10).ok_or(DecimalParseError::Overflow)?;
        fractional_digits += 1;
    }

    Ok(raw)
}
