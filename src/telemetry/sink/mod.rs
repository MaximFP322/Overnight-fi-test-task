mod file;

use std::time::Duration;

pub use file::FileTelemetrySink;

pub trait TelemetrySink {
    type Record;
    type Error;

    fn write(&mut self, record: &Self::Record) -> Result<(), Self::Error>;
    fn on_interval(&mut self, elapsed: Duration);
    fn flush(&mut self) -> Result<(), Self::Error>;
}
