use crate::market::{FeedStatus, MarketEvent};

#[derive(Debug, Clone)]
pub enum EngineIn {
    Market(MarketEvent),
    FeedStatus(FeedStatus),
}
