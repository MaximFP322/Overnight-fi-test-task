use crate::values::DecimalParseError;

#[derive(Debug)]
pub enum BinanceParseError {
    Json(sonic_rs::Error),
    UnexpectedEventType(String),
    UnexpectedSymbol {
        expected: String,
        received: String,
    },
    InvalidUpdateRange {
        first: u64,
        last: u64,
        previous: u64,
    },
    TimestampOverflow {
        milliseconds: u64,
    },
    InvalidLevel {
        field: BinanceLevelField,
        value: String,
        reason: DecimalParseError,
    },
    ZeroPrice {
        value: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinanceLevelField {
    Price,
    Quantity,
}
