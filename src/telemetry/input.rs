use crate::engine::EngineTelemetry;
use crate::message::EngineMessage;

#[derive(Debug, Clone)]
pub enum TelemetryIn {
    Engine(EngineMessage<EngineTelemetry>),
}

impl From<EngineMessage<EngineTelemetry>> for TelemetryIn {
    fn from(message: EngineMessage<EngineTelemetry>) -> Self {
        Self::Engine(message)
    }
}
