use std::convert::Infallible;

use crate::message::{ActorMessage, EngineMessage};

use super::super::state::MarketState;
use super::super::strategy::StrategyRuntime;
use super::super::{EngineIn, EngineOut, EngineOutput, OrderBook};
use super::EngineProcessor;

pub struct MarketProcessor<BOOK> {
    state: MarketState<BOOK>,
    strategy: StrategyRuntime,
}

impl<BOOK> MarketProcessor<BOOK> {
    pub const fn new(state: MarketState<BOOK>, strategy: StrategyRuntime) -> Self {
        Self { state, strategy }
    }

    pub fn state(&self) -> &MarketState<BOOK> {
        &self.state
    }
}

impl<BOOK: OrderBook> EngineProcessor for MarketProcessor<BOOK> {
    type Error = Infallible;

    fn process_message(&mut self, message: EngineMessage<EngineIn>, outgoing: &mut EngineOutput) -> Result<(), Self::Error> {
        outgoing[0] = None;
        outgoing[1] = None;
        let EngineMessage {
            actor: ActorMessage { meta, payload },
            engine_ts_ns,
            engine_seq,
        } = message;
        let request = match payload {
            EngineIn::Market(event) => self.state.apply_market(event, meta.ts.received_ts_ns),
            EngineIn::FeedStatus(status) => {
                self.state.apply_status(status);
                None
            }
        };
        outgoing[0] = if let Some(request) = request {
            let message = EngineMessage {
                actor: ActorMessage { meta, payload: request },
                engine_ts_ns,
                engine_seq,
            };
            Some(EngineOut::Resync(message))
        } else if let Some(signal) = self.strategy.on_state(&self.state, engine_ts_ns) {
            let message = EngineMessage {
                actor: ActorMessage { meta, payload: signal },
                engine_ts_ns,
                engine_seq,
            };
            Some(EngineOut::Signal(message))
        } else {
            None
        };
        Ok(())
    }
}
