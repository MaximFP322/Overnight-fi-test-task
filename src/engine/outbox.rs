use crate::queue::QueueSendError;

use super::EngineOut;

pub trait EngineOutbox {
    fn try_send(&mut self, outgoing: EngineOut) -> Result<(), QueueSendError>;
}
