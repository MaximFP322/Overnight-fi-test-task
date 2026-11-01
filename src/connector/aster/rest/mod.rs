mod http;

pub use http::AsterRest;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::event::AsterDepthSnapshot;
use super::parser::AsterParseError;
use crate::client::BlockingHttpClientError;

#[derive(Debug)]
pub struct ReceivedAsterSnapshot {
    pub received_ts_ns: u64,
    pub parsed_ts_ns: u64,
    pub snapshot: AsterDepthSnapshot,
}

#[derive(Debug)]
pub enum AsterRestError {
    Http(BlockingHttpClientError),
    Parse(AsterParseError),
    InvalidRetryConfig,
    Cancelled,
}

pub trait AsterDiffRest {
    type Error;

    fn fetch_snapshot(&mut self, cancellation: &Arc<AtomicBool>) -> Result<ReceivedAsterSnapshot, Self::Error>;
}
