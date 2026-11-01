use std::fmt;

use serde::de::{self, IgnoredAny, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::market::BookLevel;
use crate::values::{DecimalParseError, Price, Quantity};

use super::error::{AsterBookSide, AsterLevelField, AsterParseError};

pub(super) struct ParsedLevels(Result<Vec<BookLevel>, (usize, LevelParseError)>);

impl ParsedLevels {
    pub(super) fn into_levels(self, side: AsterBookSide) -> Result<Vec<BookLevel>, AsterParseError> {
        self.0.map_err(|(index, error)| match error {
            LevelParseError::InvalidDecimal { field, value, reason } => AsterParseError::InvalidLevel {
                side,
                index,
                field,
                value,
                reason,
            },
            LevelParseError::ZeroPrice => AsterParseError::ZeroPrice { side, index },
        })
    }
}

impl<'de> Deserialize<'de> for ParsedLevels {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct LevelsVisitor;

        impl<'de> Visitor<'de> for LevelsVisitor {
            type Value = ParsedLevels;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an array of [price, quantity] levels")
            }

            fn visit_seq<S: SeqAccess<'de>>(self, mut sequence: S) -> Result<Self::Value, S::Error> {
                let mut levels = Vec::new();
                while let Some(ParsedLevel(level)) = sequence.next_element()? {
                    match level {
                        Ok(level) => levels.push(level),
                        Err(error) => {
                            while sequence.next_element::<IgnoredAny>()?.is_some() {}
                            return Ok(ParsedLevels(Err((levels.len(), error))));
                        }
                    }
                }
                Ok(ParsedLevels(Ok(levels)))
            }
        }

        deserializer.deserialize_seq(LevelsVisitor)
    }
}

enum LevelParseError {
    InvalidDecimal {
        field: AsterLevelField,
        value: String,
        reason: DecimalParseError,
    },
    ZeroPrice,
}

struct ParsedLevel(Result<BookLevel, LevelParseError>);

impl<'de> Deserialize<'de> for ParsedLevel {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct LevelVisitor;

        impl<'de> Visitor<'de> for LevelVisitor {
            type Value = ParsedLevel;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("exactly two decimal strings: [price, quantity]")
            }

            fn visit_seq<S: SeqAccess<'de>>(self, mut sequence: S) -> Result<Self::Value, S::Error> {
                let price: &str = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let quantity: &str = sequence
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                if sequence.next_element::<IgnoredAny>()?.is_some() {
                    return Err(de::Error::invalid_length(3, &self));
                }

                let level = (|| {
                    let price = parse_price(price)?;
                    let quantity = parse_quantity(quantity)?;
                    Ok(BookLevel { price, quantity })
                })();
                Ok(ParsedLevel(level))
            }
        }

        deserializer.deserialize_seq(LevelVisitor)
    }
}

fn parse_price(value: &str) -> Result<Price, LevelParseError> {
    let price = Price::try_from(value).map_err(|reason| LevelParseError::InvalidDecimal {
        field: AsterLevelField::Price,
        value: value.to_owned(),
        reason,
    })?;
    if price.raw() == 0 {
        return Err(LevelParseError::ZeroPrice);
    }
    Ok(price)
}

fn parse_quantity(value: &str) -> Result<Quantity, LevelParseError> {
    Quantity::try_from(value).map_err(|reason| LevelParseError::InvalidDecimal {
        field: AsterLevelField::Quantity,
        value: value.to_owned(),
        reason,
    })
}
