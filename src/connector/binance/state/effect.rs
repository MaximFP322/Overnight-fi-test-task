use crate::connector::ConnectorOut;
use crate::instrument::InstrumentId;

use super::BinanceSyncError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BinanceAction {
    #[default]
    None,
    FetchSnapshot,
    Resync {
        reason: BinanceSyncError,
    },
    Synchronized {
        snapshot_id: u64,
        last_update_id: u64,
    },
    IgnoredInstrument {
        instrument_id: InstrumentId,
    },
}

pub type BinanceOutput = [Option<ConnectorOut>; 2];
