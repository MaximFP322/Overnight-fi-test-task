pub trait ActorSender<T> {
    fn try_send(&mut self, message: T) -> Result<(), QueueSendError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueSendError {
    Full,
    Closed,
}
