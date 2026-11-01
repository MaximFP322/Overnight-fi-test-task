use crate::market::{BookDelta, BookSnapshot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsterDepthDelta {
    pub event_ts_ns: u64,
    pub transaction_ts_ns: u64,
    pub book: BookDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsterDepthSnapshot {
    pub event_ts_ns: u64,
    pub transaction_ts_ns: u64,
    pub book: BookSnapshot,
}
