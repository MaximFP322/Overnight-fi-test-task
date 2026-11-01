use std::convert::Infallible;

use crate::client::BlockingHttpClientError;
use crate::engine::{EngineActorError, PagedOrderBookError};
use crate::market::Venue;
use crate::telemetry::TelemetryActorError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorKind {
    BinanceConnector,
    AsterConnector,
    Engine,
    SignalOutput,
    Telemetry,
}

#[derive(Debug)]
pub enum ConnectorTaskError<E> {
    Run(E),
    Shutdown(E),
    RunAndShutdown { run: E, shutdown: E },
}

#[derive(Debug)]
pub enum PipelineError<B, A, E> {
    BinanceConnector(Box<ConnectorTaskError<B>>),
    AsterConnector(Box<ConnectorTaskError<A>>),
    Engine(EngineActorError<E>),
    Telemetry(TelemetryActorError<Infallible, std::io::Error>),
    TelemetryFileOpen(std::io::Error),
    ConnectorCreate { actor: ActorKind, source: BlockingHttpClientError },
    OrderBookCreate { venue: Venue, source: PagedOrderBookError },
    ThreadStart { actor: ActorKind, source: std::io::Error },
    ThreadPanicked { actor: ActorKind },
    UnexpectedExit(ActorKind),
}
