use super::sync::AsterSyncError;

#[derive(Debug)]
pub enum AsterStateError {
    Sync(AsterSyncError),
    SequenceExhausted,
}
