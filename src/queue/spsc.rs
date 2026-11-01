use std::num::NonZeroUsize;

use rtrb::{Consumer, PopError, Producer, PushError, RingBuffer};

use super::{ActorReceiver, ActorRecvResult, ActorSender, QueueSendError};

pub fn bounded_actor_channel<T>(capacity: NonZeroUsize) -> (SpscSender<T>, SpscReceiver<T>) {
    let (sender, receiver) = RingBuffer::new(capacity.get());
    (SpscSender { sender }, SpscReceiver { receiver })
}

#[derive(Debug)]
pub struct SpscSender<T> {
    sender: Producer<T>,
}

#[derive(Debug)]
pub struct SpscReceiver<T> {
    receiver: Consumer<T>,
}

impl<T> ActorSender<T> for SpscSender<T> {
    fn try_send(&mut self, message: T) -> Result<(), QueueSendError> {
        if self.sender.is_abandoned() {
            return Err(QueueSendError::Closed);
        }
        self.sender.push(message).map_err(|error| match error {
            PushError::Full(_) => QueueSendError::Full,
        })
    }
}

impl<T> ActorReceiver<T> for SpscReceiver<T> {
    fn try_recv(&mut self) -> ActorRecvResult<T> {
        match self.receiver.pop() {
            Ok(message) => ActorRecvResult::Message(message),
            Err(PopError::Empty) => ActorRecvResult::Empty,
        }
    }
}
