use serde::Serialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ActorMeta {
    pub ts: ActorTimestamps,
    pub seq: ActorSequences,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ActorTimestamps {
    pub exchange_ts_ns: Option<u64>,
    pub source_ts_ns: Option<u64>,
    pub received_ts_ns: Option<u64>,
    pub actor_ts_ns: Option<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ActorSequences {
    pub stream_epoch: Option<u64>,
    pub stream_seq: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActorMessage<T> {
    pub meta: ActorMeta,
    pub payload: T,
}

impl<T> ActorMessage<T> {
    pub fn map<U>(self, transform: impl FnOnce(T) -> U) -> ActorMessage<U> {
        ActorMessage {
            meta: self.meta,
            payload: transform(self.payload),
        }
    }
}
