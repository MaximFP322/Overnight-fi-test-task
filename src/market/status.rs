use crate::instrument::InstrumentId;

use super::Venue;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FeedState {
    #[default]
    Disconnected,
    Connecting,
    Syncing,
    Live,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedStatus {
    pub venue: Venue,
    pub instrument_id: InstrumentId,
    pub state: FeedState,
}
