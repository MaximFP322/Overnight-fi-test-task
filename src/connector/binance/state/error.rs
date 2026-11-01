use super::sync::BinanceSyncError;

#[derive(Debug)]
pub enum BinanceStateError {
    Sync(BinanceSyncError),
    SequenceExhausted,
}
