use crate::connector::ConnectorIn;
use crate::engine::strategy::Signal;
use crate::engine::{EngineOut, EngineOutbox as EngineOutboxContract, ResyncRequest};
use crate::market::Venue;
use crate::message::EngineMessage;
use crate::queue::{ActorSender, QueueSendError};
use crate::telemetry::TelemetryIn;

pub struct EngineOutbox<SIGNAL, BINANCE, ASTER, TELEMETRY> {
    signal_sender: SIGNAL,
    binance_sender: BINANCE,
    aster_sender: ASTER,
    telemetry_sender: TELEMETRY,
}

impl<SIGNAL, BINANCE, ASTER, TELEMETRY> EngineOutbox<SIGNAL, BINANCE, ASTER, TELEMETRY> {
    pub const fn new(signal_sender: SIGNAL, binance_sender: BINANCE, aster_sender: ASTER, telemetry_sender: TELEMETRY) -> Self {
        Self {
            signal_sender,
            binance_sender,
            aster_sender,
            telemetry_sender,
        }
    }
}

impl<SIGNAL, BINANCE, ASTER, TELEMETRY> EngineOutboxContract for EngineOutbox<SIGNAL, BINANCE, ASTER, TELEMETRY>
where
    SIGNAL: ActorSender<EngineMessage<Signal>>,
    BINANCE: ActorSender<ConnectorIn>,
    ASTER: ActorSender<ConnectorIn>,
    TELEMETRY: ActorSender<TelemetryIn>,
{
    fn try_send(&mut self, outgoing: EngineOut) -> Result<(), QueueSendError> {
        match outgoing {
            EngineOut::Signal(message) => self.signal_sender.try_send(message),
            EngineOut::Telemetry(message) => self.telemetry_sender.try_send(message.into()),
            EngineOut::Resync(message) => {
                let request = message.actor.payload;
                match request.venue {
                    Venue::Binance => self.binance_sender.try_send(request.into()),
                    Venue::Aster => self.aster_sender.try_send(request.into()),
                }
            }
        }
    }
}

impl From<ResyncRequest> for ConnectorIn {
    fn from(request: ResyncRequest) -> Self {
        Self::Resync {
            instrument_id: request.instrument_id,
        }
    }
}
