use crate::engine::{MarketState, OrderBook};

use super::{CrossVenueSpread, Signal};

#[derive(Debug)]
pub enum StrategyRuntime {
    CrossVenueSpread(CrossVenueSpread),
}

impl StrategyRuntime {
    pub fn on_state(&mut self, state: &MarketState<impl OrderBook>, now_ns: u64) -> Option<Signal> {
        match self {
            Self::CrossVenueSpread(strategy) => strategy
                .on_state(state, now_ns)
                .map(Signal::CrossVenueSpread),
        }
    }
}
