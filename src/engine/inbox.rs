use crate::message::ActorMessage;
use crate::queue::ActorRecvResult;

use super::EngineIn;

pub trait EngineInbox {
    fn try_recv(&mut self) -> ActorRecvResult<ActorMessage<EngineIn>>;
}
