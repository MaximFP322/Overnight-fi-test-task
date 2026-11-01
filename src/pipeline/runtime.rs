use std::fs::OpenOptions;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::clock::Clock;
use crate::connector::aster::{AsterDiffConnectorActor, AsterProcessor};
use crate::connector::binance::{BinanceDiffConnectorActor, BinanceProcessor};
use crate::engine::{EngineActor, EngineMetrics, EngineProcessor};
use crate::telemetry::{FileTelemetrySink, MetricsProcessor, TelemetryActor};

use super::connector::{run_aster_until_cancelled, run_binance_until_cancelled};
use super::output::run_signal_output;
use super::supervisor::Supervisor;
use super::{ConnectorPorts, EnginePorts, PipelineChannels, PipelineConfig, PipelineError, SignalInbox, TelemetryInbox};

type PipelineResult<BINANCE, ASTER, PROCESSOR> =
    Result<(), PipelineError<<BINANCE as BinanceProcessor>::Error, <ASTER as AsterProcessor>::Error, <PROCESSOR as EngineProcessor>::Error>>;

pub fn run_until_shutdown<BINANCE, ASTER, PROCESSOR, CLOCK>(
    config: PipelineConfig,
    binance: BINANCE,
    aster: ASTER,
    processor: PROCESSOR,
    clock: CLOCK,
    shutdown: Arc<AtomicBool>,
) -> PipelineResult<BINANCE, ASTER, PROCESSOR>
where
    BINANCE: BinanceProcessor + Send + 'static,
    BINANCE::Error: std::fmt::Debug + Send + 'static,
    ASTER: AsterProcessor + Send + 'static,
    ASTER::Error: std::fmt::Debug + Send + 'static,
    PROCESSOR: EngineProcessor + Send + 'static,
    PROCESSOR::Error: std::fmt::Debug + Send + 'static,
    CLOCK: Clock + Send + 'static,
{
    let mut supervisor = Supervisor::new(shutdown);
    let channels = PipelineChannels::new(&config, supervisor.cancellation());
    start_engine(processor, clock, channels.engine, &mut supervisor)?;
    start_signal_output(channels.signals, &mut supervisor)?;
    start_telemetry(&config.telemetry_path, channels.telemetry, &mut supervisor)?;
    start_binance(binance, channels.binance, &mut supervisor)?;
    start_aster(aster, channels.aster, &mut supervisor)?;
    supervisor.wait()
}

pub(crate) fn start_binance<PROCESSOR, A, E>(
    processor: PROCESSOR,
    ports: ConnectorPorts,
    supervisor: &mut Supervisor<PROCESSOR::Error, A, E>,
) -> Result<(), PipelineError<PROCESSOR::Error, A, E>>
where
    PROCESSOR: BinanceProcessor + Send + 'static,
    PROCESSOR::Error: std::fmt::Debug + Send + 'static,
    A: std::fmt::Debug + Send + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    let (inbox, outbox) = ports.into_parts();
    let actor = BinanceDiffConnectorActor::new(processor, inbox, outbox);
    supervisor.start_binance(move |cancellation| run_binance_until_cancelled(actor, cancellation))
}

pub(crate) fn start_aster<B, PROCESSOR, E>(
    processor: PROCESSOR,
    ports: ConnectorPorts,
    supervisor: &mut Supervisor<B, PROCESSOR::Error, E>,
) -> Result<(), PipelineError<B, PROCESSOR::Error, E>>
where
    PROCESSOR: AsterProcessor + Send + 'static,
    PROCESSOR::Error: std::fmt::Debug + Send + 'static,
    B: std::fmt::Debug + Send + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    let (inbox, outbox) = ports.into_parts();
    let actor = AsterDiffConnectorActor::new(processor, inbox, outbox);
    supervisor.start_aster(move |cancellation| run_aster_until_cancelled(actor, cancellation))
}

pub(crate) fn start_engine<B, A, PROCESSOR, CLOCK>(
    processor: PROCESSOR,
    clock: CLOCK,
    ports: EnginePorts,
    supervisor: &mut Supervisor<B, A, PROCESSOR::Error>,
) -> Result<(), PipelineError<B, A, PROCESSOR::Error>>
where
    PROCESSOR: EngineProcessor + Send + 'static,
    PROCESSOR::Error: std::fmt::Debug + Send + 'static,
    CLOCK: Clock + Send + 'static,
    B: std::fmt::Debug + Send + 'static,
    A: std::fmt::Debug + Send + 'static,
{
    let (inbox, outbox) = ports.into_parts();
    let mut actor = EngineActor::new(processor, inbox, outbox, clock, EngineMetrics::new());
    supervisor.start_engine(move |_cancellation| actor.run())
}

pub(crate) fn start_signal_output<B, A, E>(inbox: SignalInbox, supervisor: &mut Supervisor<B, A, E>) -> Result<(), PipelineError<B, A, E>>
where
    B: std::fmt::Debug + Send + 'static,
    A: std::fmt::Debug + Send + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    supervisor.start_signal_output(move |cancellation| {
        run_signal_output(inbox, &cancellation);
        Ok(())
    })
}

pub(crate) fn start_telemetry<B, A, E>(path: &Path, inbox: TelemetryInbox, supervisor: &mut Supervisor<B, A, E>) -> Result<(), PipelineError<B, A, E>>
where
    B: std::fmt::Debug + Send + 'static,
    A: std::fmt::Debug + Send + 'static,
    E: std::fmt::Debug + Send + 'static,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(PipelineError::TelemetryFileOpen)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(PipelineError::TelemetryFileOpen)?;
    let mut actor = TelemetryActor::new(MetricsProcessor::new(), inbox, FileTelemetrySink::new(file));
    supervisor.start_telemetry(move |producers_done| actor.run(&producers_done))
}
