use crate::instrument::InstrumentId;
use crate::market::Venue;
use crate::message::EngineMessage;

use super::EngineTelemetry;
use super::strategy::Signal;

#[derive(Debug, Clone)]
pub enum EngineOut {
    Signal(EngineMessage<Signal>),
    Resync(EngineMessage<ResyncRequest>),
    Telemetry(EngineMessage<EngineTelemetry>),
}

pub type EngineOutput = [Option<EngineOut>; 2];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResyncRequest {
    pub venue: Venue,
    pub instrument_id: InstrumentId,
}
