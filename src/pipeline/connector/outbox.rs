use crate::connector::{ConnectorOut, ConnectorOutbox as ConnectorOutboxContract};
use crate::engine::EngineIn;
use crate::message::ActorMessage;
use crate::queue::{ActorSender, QueueSendError};

pub struct ConnectorOutbox<SENDER> {
    engine_sender: SENDER,
}

impl<SENDER> ConnectorOutbox<SENDER> {
    pub const fn new(engine_sender: SENDER) -> Self {
        Self { engine_sender }
    }
}

impl<SENDER: ActorSender<ActorMessage<EngineIn>>> ConnectorOutboxContract for ConnectorOutbox<SENDER> {
    fn try_send(&mut self, outgoing: ConnectorOut) -> Result<(), QueueSendError> {
        self.engine_sender.try_send(outgoing.into())
    }
}

impl From<ConnectorOut> for ActorMessage<EngineIn> {
    fn from(outgoing: ConnectorOut) -> Self {
        match outgoing {
            ConnectorOut::Market(message) => message.map(EngineIn::Market),
            ConnectorOut::FeedStatus(message) => message.map(EngineIn::FeedStatus),
        }
    }
}
