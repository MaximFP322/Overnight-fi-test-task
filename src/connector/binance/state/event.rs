use crate::connector::ConnectorIn;
use crate::message::ActorMessage;

use super::super::{BinanceDepthDelta, ReceivedBinanceSnapshot};

#[derive(Debug)]
pub enum BinanceEvent {
    SessionStarted,
    Connected,
    Snapshot(ReceivedBinanceSnapshot),
    Delta(ActorMessage<BinanceDepthDelta>),
    Command(ConnectorIn),
    Disconnected,
}
