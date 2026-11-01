use crate::instrument::InstrumentId;
use crate::values::{Price, Quantity};

use super::Venue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookLevel {
    pub price: Price,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookSnapshot {
    pub venue: Venue,
    pub instrument_id: InstrumentId,
    pub last_update_id: u64,
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BookDelta {
    pub venue: Venue,
    pub instrument_id: InstrumentId,
    pub first_update_id: u64,
    pub final_update_id: u64,
    pub previous_final_update_id: Option<u64>,
    pub bids: Vec<BookLevel>,
    pub asks: Vec<BookLevel>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarketEvent {
    Snapshot(BookSnapshot),
    Delta(BookDelta),
}
