use std::str::FromStr;

use super::{DecimalParseError, parse_decimal_1e8};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Price(i64);

impl Price {
    pub const fn raw(self) -> i64 {
        self.0
    }
}

impl From<i64> for Price {
    fn from(raw: i64) -> Self {
        Self(raw)
    }
}

impl From<Price> for i64 {
    fn from(value: Price) -> Self {
        value.0
    }
}

impl TryFrom<&str> for Price {
    type Error = DecimalParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_decimal_1e8(value).map(Self)
    }
}

impl FromStr for Price {
    type Err = DecimalParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Quantity(i64);

impl Quantity {
    pub const fn raw(self) -> i64 {
        self.0
    }
}

impl From<i64> for Quantity {
    fn from(raw: i64) -> Self {
        Self(raw)
    }
}

impl From<Quantity> for i64 {
    fn from(value: Quantity) -> Self {
        value.0
    }
}

impl TryFrom<&str> for Quantity {
    type Error = DecimalParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        parse_decimal_1e8(value).map(Self)
    }
}

impl FromStr for Quantity {
    type Err = DecimalParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::try_from(value)
    }
}
