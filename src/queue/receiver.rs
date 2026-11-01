pub trait ActorReceiver<T> {
    fn try_recv(&mut self) -> ActorRecvResult<T>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorRecvResult<T> {
    Message(T),
    Empty,
    Closed,
}
