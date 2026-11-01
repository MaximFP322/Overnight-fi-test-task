use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use overnight_fi_test_task::engine::strategy::{CrossVenueSpread, StrategyRuntime};
use overnight_fi_test_task::pipeline::connector::{AsterConnector, BinanceConnector};
use overnight_fi_test_task::pipeline::{Engine, LivePipelineError, Pipeline, PipelineChannels, PipelineConfig, SignalOutput, Telemetry};

fn main() -> ExitCode {
    tracing_subscriber::fmt().with_target(false).init();

    run_application()
}

fn run_application() -> ExitCode {
    let shutdown = Arc::new(AtomicBool::new(false));
    if let Err(error) = signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&shutdown)) {
        tracing::error!(?error, "Ctrl+C handler registration failed");
        return ExitCode::FAILURE;
    }

    match run_pipeline(PipelineConfig::default(), shutdown) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = ?error, "pipeline failed");
            ExitCode::FAILURE
        }
    }
}

fn run_pipeline(config: PipelineConfig, shutdown: Arc<AtomicBool>) -> Result<(), LivePipelineError> {
    let mut pipeline = Pipeline::new(shutdown);
    let channels = PipelineChannels::new(&config, pipeline.cancellation());
    let strategy = StrategyRuntime::CrossVenueSpread(CrossVenueSpread::new(1, 5_000_000_000));

    Engine::run(&config.orderbook, strategy, channels.engine, &mut pipeline)?;
    SignalOutput::run(channels.signals, &mut pipeline)?;
    Telemetry::run(&config.telemetry_path, channels.telemetry, &mut pipeline)?;
    BinanceConnector::run(&config, channels.binance, &mut pipeline)?;
    AsterConnector::run(&config, channels.aster, &mut pipeline)?;

    tracing::info!(telemetry_path = %config.telemetry_path.display(), "busy-spin actor pipeline started; Ctrl+C stops the pipeline");
    pipeline.wait()
}
