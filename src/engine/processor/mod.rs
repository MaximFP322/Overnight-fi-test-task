mod market;

use crate::message::EngineMessage;

use super::{EngineIn, EngineOutput};

pub use market::MarketProcessor;

pub trait EngineProcessor {
    type Error;

    fn process_message(&mut self, message: EngineMessage<EngineIn>, outgoing: &mut EngineOutput) -> Result<(), Self::Error>;
}
