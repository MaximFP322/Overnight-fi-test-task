use std::fmt;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::{FormatTime, SystemTime};

use crate::client::HttpClientConfig;
use crate::connector::RestRetryConfig;
use crate::connector::aster::AsterDiffConfig;
use crate::connector::binance::BinanceDiffConfig;
use crate::values::Price;

#[derive(Debug, Clone)]
pub struct OrderBookConfig {
    pub binance_tick_size: Price,
    pub aster_tick_size: Price,
    pub reserved_pages: usize,
}

impl Default for OrderBookConfig {
    fn default() -> Self {
        Self {
            binance_tick_size: Price::from(10_000_000),
            aster_tick_size: Price::from(10_000_000),
            reserved_pages: 1024,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub http_client: HttpClientConfig,
    pub rest_retry: RestRetryConfig,
    pub binance: BinanceDiffConfig,
    pub aster: AsterDiffConfig,
    pub orderbook: OrderBookConfig,
    pub engine_queue_capacity: NonZeroUsize,
    pub signal_queue_capacity: NonZeroUsize,
    pub telemetry_queue_capacity: NonZeroUsize,
    pub telemetry_path: PathBuf,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            http_client: HttpClientConfig::default(),
            rest_retry: RestRetryConfig::default(),
            binance: BinanceDiffConfig::default(),
            aster: AsterDiffConfig::default(),
            orderbook: OrderBookConfig::default(),
            engine_queue_capacity: NonZeroUsize::MIN.saturating_add(1023),
            signal_queue_capacity: NonZeroUsize::MIN.saturating_add(63),
            telemetry_queue_capacity: NonZeroUsize::MIN.saturating_add(63),
            telemetry_path: PathBuf::from("logs")
                .join(format!("{}-{}", format!("{RunTimestamp}").replace(':', "-"), std::process::id()))
                .join("metrics.jsonl"),
        }
    }
}

struct RunTimestamp;

impl fmt::Display for RunTimestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        SystemTime.format_time(&mut Writer::new(f))
    }
}
