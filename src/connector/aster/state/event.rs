use crate::connector::ConnectorIn;
use crate::message::ActorMessage;

use super::super::{AsterDepthDelta, ReceivedAsterSnapshot};

#[derive(Debug)]
pub enum AsterEvent {
    SessionStarted,
    Connected,
    Snapshot(ReceivedAsterSnapshot),
    Delta(ActorMessage<AsterDepthDelta>),
    Command(ConnectorIn),
    Disconnected,
}
