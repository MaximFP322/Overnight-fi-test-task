use std::fmt;

use serde::de::{self, IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::instrument::InstrumentId;
use crate::market::{BookDelta, BookSnapshot, Venue};

use super::super::{AsterDepthDelta, AsterDepthSnapshot, AsterSymbol};
use super::error::{AsterBookSide, AsterParseError};
use super::levels::ParsedLevels;

pub fn parse_delta(bytes: &[u8], expected_symbol: &AsterSymbol, instrument_id: InstrumentId) -> Result<AsterDepthDelta, AsterParseError> {
    let DeltaPayload(dto) = sonic_rs::from_slice(bytes).map_err(AsterParseError::Json)?;
    finish_delta(dto, expected_symbol, instrument_id)
}

pub fn parse_delta_str(text: &str, expected_symbol: &AsterSymbol, instrument_id: InstrumentId) -> Result<AsterDepthDelta, AsterParseError> {
    let DeltaPayload(dto) = sonic_rs::from_str(text).map_err(AsterParseError::Json)?;
    finish_delta(dto, expected_symbol, instrument_id)
}

fn finish_delta(dto: DeltaDto<'_>, expected_symbol: &AsterSymbol, instrument_id: InstrumentId) -> Result<AsterDepthDelta, AsterParseError> {
    if dto.event_type != "depthUpdate" {
        return Err(AsterParseError::UnexpectedEventType(dto.event_type.to_owned()));
    }
    if !dto.symbol.eq_ignore_ascii_case(expected_symbol.as_str()) {
        return Err(AsterParseError::UnexpectedSymbol {
            expected: expected_symbol.as_str().to_owned(),
            received: dto.symbol.to_owned(),
        });
    }
    if dto.first_update_id > dto.final_update_id || dto.previous_final_update_id > dto.final_update_id {
        return Err(AsterParseError::InvalidUpdateRange {
            first: dto.first_update_id,
            last: dto.final_update_id,
            previous: dto.previous_final_update_id,
        });
    }

    Ok(AsterDepthDelta {
        event_ts_ns: to_ns(dto.event_ts_ms)?,
        transaction_ts_ns: to_ns(dto.transaction_ts_ms)?,
        book: BookDelta {
            venue: Venue::Aster,
            instrument_id,
            first_update_id: dto.first_update_id,
            final_update_id: dto.final_update_id,
            previous_final_update_id: Some(dto.previous_final_update_id),
            bids: dto.bids.into_levels(AsterBookSide::Bid)?,
            asks: dto.asks.into_levels(AsterBookSide::Ask)?,
        },
    })
}

pub fn parse_snapshot(bytes: &[u8], instrument_id: InstrumentId) -> Result<AsterDepthSnapshot, AsterParseError> {
    let dto: SnapshotDto = sonic_rs::from_slice(bytes).map_err(AsterParseError::Json)?;
    let bids = dto.bids.into_levels(AsterBookSide::Bid)?;
    let asks = dto.asks.into_levels(AsterBookSide::Ask)?;

    Ok(AsterDepthSnapshot {
        event_ts_ns: to_ns(dto.event_ts_ms)?,
        transaction_ts_ns: to_ns(dto.transaction_ts_ms)?,
        book: BookSnapshot {
            venue: Venue::Aster,
            instrument_id,
            last_update_id: dto.last_update_id,
            bids,
            asks,
        },
    })
}

fn to_ns(milliseconds: u64) -> Result<u64, AsterParseError> {
    milliseconds
        .checked_mul(1_000_000)
        .ok_or(AsterParseError::TimestampOverflow { milliseconds })
}

#[derive(Deserialize)]
struct DeltaDto<'a> {
    #[serde(rename = "e")]
    event_type: &'a str,
    #[serde(rename = "E")]
    event_ts_ms: u64,
    #[serde(rename = "T")]
    transaction_ts_ms: u64,
    #[serde(rename = "s")]
    symbol: &'a str,
    #[serde(rename = "U")]
    first_update_id: u64,
    #[serde(rename = "u")]
    final_update_id: u64,
    #[serde(rename = "pu")]
    previous_final_update_id: u64,
    #[serde(rename = "b")]
    bids: ParsedLevels,
    #[serde(rename = "a")]
    asks: ParsedLevels,
}

#[derive(Deserialize)]
struct SnapshotDto {
    #[serde(rename = "lastUpdateId")]
    last_update_id: u64,
    #[serde(rename = "E")]
    event_ts_ms: u64,
    #[serde(rename = "T")]
    transaction_ts_ms: u64,
    bids: ParsedLevels,
    asks: ParsedLevels,
}

struct DeltaPayload<'a>(DeltaDto<'a>);

impl<'de> Deserialize<'de> for DeltaPayload<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PayloadVisitor;

        impl<'de> Visitor<'de> for PayloadVisitor {
            type Value = DeltaPayload<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an Aster raw or combined depth event")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut fields = DeltaFields::default();
                let mut data = None;
                let mut raw_seen = false;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "data" => assign_field(&mut data, map.next_value()?, "data")?,
                        "e" => {
                            raw_seen = true;
                            assign_field(&mut fields.event_type, map.next_value()?, "e")?;
                        }
                        "E" => {
                            raw_seen = true;
                            assign_field(&mut fields.event_ts_ms, map.next_value()?, "E")?;
                        }
                        "T" => {
                            raw_seen = true;
                            assign_field(&mut fields.transaction_ts_ms, map.next_value()?, "T")?;
                        }
                        "s" => {
                            raw_seen = true;
                            assign_field(&mut fields.symbol, map.next_value()?, "s")?;
                        }
                        "U" => {
                            raw_seen = true;
                            assign_field(&mut fields.first_update_id, map.next_value()?, "U")?;
                        }
                        "u" => {
                            raw_seen = true;
                            assign_field(&mut fields.final_update_id, map.next_value()?, "u")?;
                        }
                        "pu" => {
                            raw_seen = true;
                            assign_field(&mut fields.previous_final_update_id, map.next_value()?, "pu")?;
                        }
                        "b" => {
                            raw_seen = true;
                            assign_field(&mut fields.bids, map.next_value()?, "b")?;
                        }
                        "a" => {
                            raw_seen = true;
                            assign_field(&mut fields.asks, map.next_value()?, "a")?;
                        }

                        _unknown_field => {
                            map.next_value::<IgnoredAny>()?;
                        }
                    }
                }

                match data {
                    Some(_payload) if raw_seen => Err(de::Error::custom("depth event mixes raw fields with a combined data payload")),
                    Some(payload) => Ok(DeltaPayload(payload)),
                    None => fields.finish().map(DeltaPayload),
                }
            }
        }

        deserializer.deserialize_map(PayloadVisitor)
    }
}

#[derive(Default)]
struct DeltaFields<'a> {
    event_type: Option<&'a str>,
    event_ts_ms: Option<u64>,
    transaction_ts_ms: Option<u64>,
    symbol: Option<&'a str>,
    first_update_id: Option<u64>,
    final_update_id: Option<u64>,
    previous_final_update_id: Option<u64>,
    bids: Option<ParsedLevels>,
    asks: Option<ParsedLevels>,
}

impl<'a> DeltaFields<'a> {
    fn finish<E: de::Error>(self) -> Result<DeltaDto<'a>, E> {
        Ok(DeltaDto {
            event_type: self.event_type.ok_or_else(|| E::missing_field("e"))?,
            event_ts_ms: self.event_ts_ms.ok_or_else(|| E::missing_field("E"))?,
            transaction_ts_ms: self
                .transaction_ts_ms
                .ok_or_else(|| E::missing_field("T"))?,
            symbol: self.symbol.ok_or_else(|| E::missing_field("s"))?,
            first_update_id: self.first_update_id.ok_or_else(|| E::missing_field("U"))?,
            final_update_id: self.final_update_id.ok_or_else(|| E::missing_field("u"))?,
            previous_final_update_id: self
                .previous_final_update_id
                .ok_or_else(|| E::missing_field("pu"))?,
            bids: self.bids.ok_or_else(|| E::missing_field("b"))?,
            asks: self.asks.ok_or_else(|| E::missing_field("a"))?,
        })
    }
}

fn assign_field<T, E: de::Error>(destination: &mut Option<T>, value: T, name: &'static str) -> Result<(), E> {
    if destination.is_some() {
        return Err(E::duplicate_field(name));
    }
    *destination = Some(value);
    Ok(())
}
