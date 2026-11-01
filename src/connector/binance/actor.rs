use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::connector::{ConnectorIn, ConnectorOutbox};
use crate::queue::{ActorReceiver, ActorRecvResult, QueueSendError};

use super::{BinanceOutput, BinanceProcessor};

pub struct BinanceDiffConnectorActor<PROCESSOR, INBOX, OUTBOX> {
    processor: PROCESSOR,
    inbox: INBOX,
    outbox: OUTBOX,
}

impl<PROCESSOR, INBOX, OUTBOX> BinanceDiffConnectorActor<PROCESSOR, INBOX, OUTBOX> {
    pub const fn new(processor: PROCESSOR, inbox: INBOX, outbox: OUTBOX) -> Self {
        Self { processor, inbox, outbox }
    }
}

impl<PROCESSOR, INBOX, OUTBOX> BinanceDiffConnectorActor<PROCESSOR, INBOX, OUTBOX>
where
    PROCESSOR: BinanceProcessor,
    INBOX: ActorReceiver<ConnectorIn>,
    OUTBOX: ConnectorOutbox,
{
    pub fn run(&mut self, cancellation: &Arc<AtomicBool>) -> Result<(), PROCESSOR::Error> {
        let mut outgoing: BinanceOutput = [None, None];
        while !cancellation.load(Ordering::Relaxed) {
            match self.inbox.try_recv() {
                ActorRecvResult::Message(command) => {
                    self.processor.process_message(command, &mut outgoing)?;
                    self.publish(&mut outgoing);
                }
                ActorRecvResult::Empty => {}
                ActorRecvResult::Closed => break,
            }
            if cancellation.load(Ordering::Relaxed) {
                break;
            }
            self.processor.step(cancellation, &mut outgoing)?;
            self.publish(&mut outgoing);
        }
        self.shutdown()
    }

    pub fn shutdown(&mut self) -> Result<(), PROCESSOR::Error> {
        self.processor.shutdown()
    }

    fn publish(&mut self, outgoing: &mut BinanceOutput) {
        for message in outgoing.iter_mut().filter_map(Option::take) {
            match self.outbox.try_send(message) {
                Ok(()) => {}
                Err(QueueSendError::Full) => {
                    tracing::warn!(venue = "Binance", "connector outgoing queue full; message dropped");
                }
                Err(QueueSendError::Closed) => {
                    tracing::warn!(venue = "Binance", "connector outgoing queue closed; message dropped");
                }
            }
        }
    }
}
