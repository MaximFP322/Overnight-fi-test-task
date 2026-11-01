use crate::connector::ConnectorOut;
use crate::instrument::InstrumentId;

use super::AsterSyncError;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AsterAction {
    #[default]
    None,
    FetchSnapshot,
    Resync {
        reason: AsterSyncError,
    },
    Synchronized {
        snapshot_id: u64,
        last_update_id: u64,
    },
    IgnoredInstrument {
        instrument_id: InstrumentId,
    },
}

pub type AsterOutput = [Option<ConnectorOut>; 2];
