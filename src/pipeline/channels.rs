use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::connector::ConnectorIn;
use crate::engine::EngineIn;
use crate::engine::strategy::Signal;
use crate::message::{ActorMessage, EngineMessage};
use crate::queue::{SpscReceiver, SpscSender, bounded_actor_channel};
use crate::telemetry::TelemetryIn;

use super::PipelineConfig;
use super::connector::ConnectorOutbox;
use super::engine::{EngineInbox, EngineOutbox};

type MarketInbox = EngineInbox<SpscReceiver<ActorMessage<EngineIn>>, SpscReceiver<ActorMessage<EngineIn>>>;
type MarketOutbox = EngineOutbox<SpscSender<EngineMessage<Signal>>, SpscSender<ConnectorIn>, SpscSender<ConnectorIn>, SpscSender<TelemetryIn>>;
type MarketSender = ConnectorOutbox<SpscSender<ActorMessage<EngineIn>>>;

pub type SignalInbox = SpscReceiver<EngineMessage<Signal>>;
pub type TelemetryInbox = SpscReceiver<TelemetryIn>;

pub struct ConnectorPorts {
    inbox: SpscReceiver<ConnectorIn>,
    outbox: MarketSender,
}

impl ConnectorPorts {
    pub(crate) fn into_parts(self) -> (SpscReceiver<ConnectorIn>, MarketSender) {
        (self.inbox, self.outbox)
    }
}

pub struct EnginePorts {
    inbox: MarketInbox,
    outbox: MarketOutbox,
}

impl EnginePorts {
    pub(crate) fn into_parts(self) -> (MarketInbox, MarketOutbox) {
        (self.inbox, self.outbox)
    }
}

pub struct PipelineChannels {
    pub binance: ConnectorPorts,
    pub aster: ConnectorPorts,
    pub engine: EnginePorts,
    pub signals: SignalInbox,
    pub telemetry: TelemetryInbox,
}

impl PipelineChannels {
    pub fn new(config: &PipelineConfig, cancellation: Arc<AtomicBool>) -> Self {
        let (signals, signal_inbox) = bounded_actor_channel(config.signal_queue_capacity);
        let (telemetry, telemetry_inbox) = bounded_actor_channel(config.telemetry_queue_capacity);
        let (binance_market, binance_inbox) = bounded_actor_channel(config.engine_queue_capacity);
        let (aster_market, aster_inbox) = bounded_actor_channel(config.engine_queue_capacity);
        let (binance_commands, binance_command_inbox) = bounded_actor_channel(NonZeroUsize::MIN);
        let (aster_commands, aster_command_inbox) = bounded_actor_channel(NonZeroUsize::MIN);

        Self {
            binance: ConnectorPorts {
                inbox: binance_command_inbox,
                outbox: ConnectorOutbox::new(binance_market),
            },
            aster: ConnectorPorts {
                inbox: aster_command_inbox,
                outbox: ConnectorOutbox::new(aster_market),
            },
            engine: EnginePorts {
                inbox: EngineInbox::new(binance_inbox, aster_inbox, cancellation),
                outbox: EngineOutbox::new(signals, binance_commands, aster_commands, telemetry),
            },
            signals: signal_inbox,
            telemetry: telemetry_inbox,
        }
    }
}
