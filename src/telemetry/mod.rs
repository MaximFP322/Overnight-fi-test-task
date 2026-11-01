mod actor;
mod input;
mod processor;
mod sink;

pub use actor::TelemetryActor;
pub use input::TelemetryIn;
pub use processor::{MetricsProcessor, TelemetryProcessor, TelemetryRecord};
pub use sink::{FileTelemetrySink, TelemetrySink};

#[derive(Debug)]
pub enum TelemetryActorError<PROCESSOR, SINK> {
    Processor(PROCESSOR),
    Sink(SINK),
}
