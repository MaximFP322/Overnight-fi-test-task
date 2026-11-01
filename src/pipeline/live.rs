use std::convert::Infallible;
use std::path::Path;

use crate::clock::SystemClock;
use crate::connector::aster::{AsterDiffActorError, AsterDiffWsClientError, AsterRestError};
use crate::connector::binance::{BinanceDiffActorError, BinanceDiffWsClientError, BinanceRestError};
use crate::engine::strategy::StrategyRuntime;
use crate::engine::{MarketProcessor, MarketState, PagedOrderBook};
use crate::market::Venue;

use super::runtime;
use super::supervisor::Supervisor;
use super::{EnginePorts, OrderBookConfig, PipelineError, SignalInbox, TelemetryInbox};

type BinanceError = BinanceDiffActorError<BinanceDiffWsClientError, BinanceRestError>;
type AsterError = AsterDiffActorError<AsterDiffWsClientError, AsterRestError>;

pub type Pipeline = Supervisor<BinanceError, AsterError, Infallible>;
pub type LivePipelineError = PipelineError<BinanceError, AsterError, Infallible>;

pub struct Engine;

impl Engine {
    pub fn run(config: &OrderBookConfig, strategy: StrategyRuntime, ports: EnginePorts, pipeline: &mut Pipeline) -> Result<(), LivePipelineError> {
        let binance = PagedOrderBook::new(config.binance_tick_size, config.reserved_pages).map_err(|source| PipelineError::OrderBookCreate {
            venue: Venue::Binance,
            source,
        })?;
        let aster = PagedOrderBook::new(config.aster_tick_size, config.reserved_pages)
            .map_err(|source| PipelineError::OrderBookCreate { venue: Venue::Aster, source })?;
        let processor = MarketProcessor::new(MarketState::new(binance, aster), strategy);
        runtime::start_engine(processor, SystemClock, ports, pipeline)
    }
}

pub struct Telemetry;

impl Telemetry {
    pub fn run(path: &Path, inbox: TelemetryInbox, pipeline: &mut Pipeline) -> Result<(), LivePipelineError> {
        runtime::start_telemetry(path, inbox, pipeline)
    }
}

pub struct SignalOutput;

impl SignalOutput {
    pub fn run(inbox: SignalInbox, pipeline: &mut Pipeline) -> Result<(), LivePipelineError> {
        runtime::start_signal_output(inbox, pipeline)
    }
}
