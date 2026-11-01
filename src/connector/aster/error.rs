use super::{AsterConfigError, AsterParseError, AsterStateError, AsterSyncError};

#[derive(Debug)]
pub enum AsterSessionError<W, R> {
    Connect(W),
    Ws(W),
    Rest(R),
    Parse(AsterParseError),
    Sync(AsterSyncError),
    SequenceExhausted,
}

impl<W, R> AsterSessionError<W, R> {
    pub(super) fn is_recoverable(&self) -> bool {
        match self {
            Self::Ws(_) | Self::Parse(_) | Self::Sync(_) => true,
            Self::Connect(_) | Self::Rest(_) | Self::SequenceExhausted => false,
        }
    }
}

impl<W, R> From<AsterStateError> for AsterSessionError<W, R> {
    fn from(error: AsterStateError) -> Self {
        match error {
            AsterStateError::Sync(error) => Self::Sync(error),
            AsterStateError::SequenceExhausted => Self::SequenceExhausted,
        }
    }
}

#[derive(Debug)]
pub enum AsterDiffActorError<W, R> {
    Config(AsterConfigError),
    Session(AsterSessionError<W, R>),
    ReconnectExhausted { attempts: u32, last: AsterSessionError<W, R> },
}
