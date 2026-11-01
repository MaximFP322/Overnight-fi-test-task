use std::convert::Infallible;

use serde::Serialize;

use crate::engine::EngineTelemetry;
use crate::message::EngineMessage;
use crate::telemetry::TelemetryIn;

use super::TelemetryProcessor;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TelemetryRecord {
    Engine(EngineMessage<EngineTelemetry>),
}

#[derive(Debug, Default)]
pub struct MetricsProcessor;

impl MetricsProcessor {
    pub const fn new() -> Self {
        Self
    }
}

impl TelemetryProcessor for MetricsProcessor {
    type Input = TelemetryIn;
    type Record = TelemetryRecord;
    type Error = Infallible;

    fn process_message(&mut self, message: Self::Input) -> Result<Option<Self::Record>, Self::Error> {
        match message {
            TelemetryIn::Engine(summary) => Ok(Some(TelemetryRecord::Engine(summary))),
        }
    }
}
