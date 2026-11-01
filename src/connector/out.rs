use crate::market::{FeedStatus, MarketEvent};
use crate::message::ActorMessage;

#[derive(Debug, Clone)]
pub enum ConnectorOut {
    Market(ActorMessage<MarketEvent>),
    FeedStatus(ActorMessage<FeedStatus>),
}
