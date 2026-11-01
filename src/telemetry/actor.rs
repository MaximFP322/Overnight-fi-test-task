use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::queue::{ActorReceiver, ActorRecvResult};

use super::{TelemetryActorError, TelemetryProcessor, TelemetrySink};

const REPORT_INTERVAL: Duration = Duration::from_secs(10);

pub struct TelemetryActor<PROCESSOR, INBOX, SINK> {
    processor: PROCESSOR,
    inbox: INBOX,
    sink: SINK,
}

impl<PROCESSOR, INBOX, SINK> TelemetryActor<PROCESSOR, INBOX, SINK> {
    pub const fn new(processor: PROCESSOR, inbox: INBOX, sink: SINK) -> Self {
        Self { processor, inbox, sink }
    }
}

impl<PROCESSOR, INBOX, SINK> TelemetryActor<PROCESSOR, INBOX, SINK>
where
    PROCESSOR: TelemetryProcessor,
    INBOX: ActorReceiver<PROCESSOR::Input>,
    SINK: TelemetrySink<Record = PROCESSOR::Record>,
{
    pub fn run(&mut self, producers_done: &AtomicBool) -> Result<(), TelemetryActorError<PROCESSOR::Error, SINK::Error>> {
        let mut interval_started = Instant::now();
        let result = loop {
            let draining = producers_done.load(Ordering::Acquire);
            let now = Instant::now();
            match self.inbox.try_recv() {
                ActorRecvResult::Message(message) => {
                    if let Err(error) = self.process_message(message) {
                        break Err(error);
                    }
                }
                ActorRecvResult::Empty if draining => break Ok(()),
                ActorRecvResult::Empty => {}
                ActorRecvResult::Closed => break Ok(()),
            }
            let elapsed = now.duration_since(interval_started);
            if elapsed >= REPORT_INTERVAL {
                self.sink.on_interval(elapsed);
                interval_started = now;
            }
        };
        let shutdown = self.shutdown();
        result.and(shutdown)
    }
}

impl<PROCESSOR, INBOX, SINK> TelemetryActor<PROCESSOR, INBOX, SINK>
where
    PROCESSOR: TelemetryProcessor,
    SINK: TelemetrySink<Record = PROCESSOR::Record>,
{
    pub fn process_message(&mut self, message: PROCESSOR::Input) -> Result<(), TelemetryActorError<PROCESSOR::Error, SINK::Error>> {
        if let Some(record) = self
            .processor
            .process_message(message)
            .map_err(TelemetryActorError::Processor)?
        {
            self.sink
                .write(&record)
                .map_err(TelemetryActorError::Sink)?;
        }
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), TelemetryActorError<PROCESSOR::Error, SINK::Error>> {
        self.sink.flush().map_err(TelemetryActorError::Sink)
    }
}
