use std::time::Instant;

use crate::clock::Clock;
use crate::message::{ActorMessage, EngineMessage};
use crate::queue::{ActorRecvResult, QueueSendError};

use super::{EngineActorError, EngineIn, EngineInbox, EngineMetrics, EngineOut, EngineOutbox, EngineOutput, EngineProcessor};

pub struct EngineActor<PROCESSOR, INBOX, OUTBOX, CLOCK> {
    processor: PROCESSOR,
    inbox: INBOX,
    outbox: OUTBOX,
    clock: CLOCK,
    metrics: EngineMetrics,
    next_engine_seq: u64,
    outgoing: EngineOutput,
}

impl<PROCESSOR, INBOX, OUTBOX, CLOCK> EngineActor<PROCESSOR, INBOX, OUTBOX, CLOCK> {
    pub const fn new(processor: PROCESSOR, inbox: INBOX, outbox: OUTBOX, clock: CLOCK, metrics: EngineMetrics) -> Self {
        Self {
            processor,
            inbox,
            outbox,
            clock,
            metrics,
            next_engine_seq: 0,
            outgoing: [None, None],
        }
    }
}

impl<PROCESSOR, INBOX, OUTBOX, CLOCK> EngineActor<PROCESSOR, INBOX, OUTBOX, CLOCK>
where
    PROCESSOR: EngineProcessor,
    INBOX: EngineInbox,
    OUTBOX: EngineOutbox,
    CLOCK: Clock,
{
    pub fn run(&mut self) -> Result<(), EngineActorError<PROCESSOR::Error>> {
        let result = loop {
            match self.inbox.try_recv() {
                ActorRecvResult::Message(message) => {
                    let now_ns = self.clock.now_ns();
                    if let Err(error) = self.process_message(message, now_ns) {
                        break Err(error);
                    }
                }
                ActorRecvResult::Empty => {}
                ActorRecvResult::Closed => break Ok(()),
            }
        };
        if let Some(final_telemetry) = self.metrics.finish() {
            match self.outbox.try_send(final_telemetry) {
                Ok(()) => {}
                Err(error @ (QueueSendError::Full | QueueSendError::Closed)) => {
                    self.metrics.record_delivery_failure();
                    tracing::warn!(?error, "engine final telemetry delivery failed; summary dropped");
                }
            }
        }
        result
    }
}

impl<PROCESSOR, INBOX, OUTBOX, CLOCK> EngineActor<PROCESSOR, INBOX, OUTBOX, CLOCK>
where
    PROCESSOR: EngineProcessor,
    OUTBOX: EngineOutbox,
{
    pub fn process_message(&mut self, message: ActorMessage<EngineIn>, now_ns: u64) -> Result<(), EngineActorError<PROCESSOR::Error>> {
        let processing_started = Instant::now();
        let engine_seq = self.next_engine_seq;
        self.next_engine_seq = engine_seq
            .checked_add(1)
            .ok_or(EngineActorError::SequenceExhausted)?;

        let meta = message.meta;
        let message = EngineMessage {
            actor: message,
            engine_ts_ns: now_ns,
            engine_seq,
        };

        self.processor
            .process_message(message, &mut self.outgoing)
            .map_err(EngineActorError::Processor)?;
        let processing_ns = processing_started.elapsed().as_nanos() as u64;
        let telemetry = self
            .metrics
            .record_message(meta, now_ns, engine_seq, processing_ns, &self.outgoing);
        self.publish(telemetry);
        Ok(())
    }

    fn publish(&mut self, telemetry: Option<EngineOut>) {
        for message in self
            .outgoing
            .iter_mut()
            .filter_map(Option::take)
            .chain(telemetry)
        {
            match self.outbox.try_send(message) {
                Ok(()) => {}
                Err(QueueSendError::Full | QueueSendError::Closed) => self.metrics.record_delivery_failure(),
            }
        }
    }
}
