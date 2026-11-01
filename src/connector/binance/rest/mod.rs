mod http;

pub use http::BinanceRest;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::event::BinanceDepthSnapshot;
use super::parser::BinanceParseError;
use crate::client::BlockingHttpClientError;

#[derive(Debug)]
pub struct ReceivedBinanceSnapshot {
    pub received_ts_ns: u64,
    pub parsed_ts_ns: u64,
    pub snapshot: BinanceDepthSnapshot,
}

#[derive(Debug)]
pub enum BinanceRestError {
    Http(BlockingHttpClientError),
    Parse(BinanceParseError),
    InvalidRetryConfig,
    Cancelled,
}

pub trait BinanceDiffRest {
    type Error;

    fn fetch_snapshot(&mut self, cancellation: &Arc<AtomicBool>) -> Result<ReceivedBinanceSnapshot, Self::Error>;
}
