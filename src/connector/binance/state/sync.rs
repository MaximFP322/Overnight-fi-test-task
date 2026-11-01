use crate::market::BookDelta;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinanceSyncError {
    SnapshotDoesNotBridge {
        snapshot: u64,
        first: u64,
        final_id: u64,
    },
    Gap {
        expected_previous: u64,
        actual_previous: Option<u64>,
        final_id: u64,
    },
    MissingSnapshot,
}

pub(super) enum SyncAction {
    Ignore,
    Bridge,
    Apply,
    Resync(BinanceSyncError),
}

pub(super) struct DepthSync {
    last_update_id: u64,
    bridged: bool,
}

impl DepthSync {
    pub(super) const fn new(snapshot_id: u64) -> Self {
        Self {
            last_update_id: snapshot_id,
            bridged: false,
        }
    }

    pub(super) const fn is_bridged(&self) -> bool {
        self.bridged
    }
    pub(super) fn apply(&mut self, delta: &BookDelta) -> SyncAction {
        if !self.bridged {
            if delta.final_update_id < self.last_update_id {
                return SyncAction::Ignore;
            }
            if delta.first_update_id > self.last_update_id {
                return SyncAction::Resync(BinanceSyncError::SnapshotDoesNotBridge {
                    snapshot: self.last_update_id,
                    first: delta.first_update_id,
                    final_id: delta.final_update_id,
                });
            }
            self.bridged = true;
            self.last_update_id = delta.final_update_id;
            return SyncAction::Bridge;
        }
        if delta.final_update_id <= self.last_update_id {
            return SyncAction::Ignore;
        }
        if delta.previous_final_update_id != Some(self.last_update_id) {
            return SyncAction::Resync(BinanceSyncError::Gap {
                expected_previous: self.last_update_id,
                actual_previous: delta.previous_final_update_id,
                final_id: delta.final_update_id,
            });
        }
        self.last_update_id = delta.final_update_id;
        SyncAction::Apply
    }
}
