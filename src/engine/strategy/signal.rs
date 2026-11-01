use super::strategies::CrossVenueSpreadSignal;

#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    CrossVenueSpread(CrossVenueSpreadSignal),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SignalBias {
    #[default]
    Hold,
    LongBias,
    ShortBias,
}
