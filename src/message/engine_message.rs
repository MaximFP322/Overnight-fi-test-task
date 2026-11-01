use serde::Serialize;

use super::ActorMessage;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EngineMessage<T> {
    pub actor: ActorMessage<T>,
    pub engine_ts_ns: u64,
    pub engine_seq: u64,
}

impl<T> EngineMessage<T> {
    pub fn map<U>(self, transform: impl FnOnce(T) -> U) -> EngineMessage<U> {
        EngineMessage {
            actor: self.actor.map(transform),
            engine_ts_ns: self.engine_ts_ns,
            engine_seq: self.engine_seq,
        }
    }
}
