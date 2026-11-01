use crate::values::DecimalParseError;

#[derive(Debug)]
pub enum AsterParseError {
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
        side: AsterBookSide,
        index: usize,
        field: AsterLevelField,
        value: String,
        reason: DecimalParseError,
    },
    ZeroPrice {
        side: AsterBookSide,
        index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsterBookSide {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsterLevelField {
    Price,
    Quantity,
}
