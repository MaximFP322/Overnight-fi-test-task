mod metrics;

pub use metrics::{MetricsProcessor, TelemetryRecord};

pub trait TelemetryProcessor {
    type Input;
    type Record;
    type Error;

    fn process_message(&mut self, message: Self::Input) -> Result<Option<Self::Record>, Self::Error>;
}
