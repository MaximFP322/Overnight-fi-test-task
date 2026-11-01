use super::{BinanceConfigError, BinanceParseError, BinanceStateError, BinanceSyncError};

#[derive(Debug)]
pub enum BinanceSessionError<W, R> {
    Connect(W),
    Ws(W),
    Rest(R),
    Parse(BinanceParseError),
    Sync(BinanceSyncError),
    SequenceExhausted,
}

impl<W, R> BinanceSessionError<W, R> {
    pub(super) fn is_recoverable(&self) -> bool {
        match self {
            Self::Ws(_) | Self::Parse(_) | Self::Sync(_) => true,
            Self::Connect(_) | Self::Rest(_) | Self::SequenceExhausted => false,
        }
    }
}

impl<W, R> From<BinanceStateError> for BinanceSessionError<W, R> {
    fn from(error: BinanceStateError) -> Self {
        match error {
            BinanceStateError::Sync(error) => Self::Sync(error),
            BinanceStateError::SequenceExhausted => Self::SequenceExhausted,
        }
    }
}

#[derive(Debug)]
pub enum BinanceDiffActorError<W, R> {
    Config(BinanceConfigError),
    Session(BinanceSessionError<W, R>),
    ReconnectExhausted { attempts: u32, last: BinanceSessionError<W, R> },
}
