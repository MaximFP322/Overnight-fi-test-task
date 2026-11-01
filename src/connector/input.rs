use crate::instrument::InstrumentId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorIn {
    Resync { instrument_id: InstrumentId },
}
