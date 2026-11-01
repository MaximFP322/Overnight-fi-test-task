use std::time::Duration;

use backon::ExponentialBuilder;

pub mod aster;
pub mod binance;
mod input;
mod out;
mod outbox;

pub use input::ConnectorIn;
pub use out::ConnectorOut;
pub use outbox::ConnectorOutbox;

#[derive(Debug, Clone)]
pub struct RestRetryConfig {
    initial_backoff: Duration,
    max_backoff: Duration,
    max_attempts: u32,
}

#[derive(Debug)]
pub struct RestRetryConfigError;

impl RestRetryConfig {
    pub fn new(initial_backoff: Duration, max_backoff: Duration, max_attempts: u32) -> Result<Self, RestRetryConfigError> {
        let config = Self {
            initial_backoff,
            max_backoff,
            max_attempts,
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), RestRetryConfigError> {
        if self.initial_backoff.is_zero() || self.max_backoff < self.initial_backoff || self.max_attempts == 0 {
            return Err(RestRetryConfigError);
        }
        Ok(())
    }

    pub const fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    pub(crate) fn backoff(&self) -> ExponentialBuilder {
        ExponentialBuilder::default()
            .with_min_delay(self.initial_backoff)
            .with_max_delay(self.max_backoff)
            .with_max_times((self.max_attempts - 1) as usize)
    }
}

impl Default for RestRetryConfig {
    fn default() -> Self {
        Self {
            initial_backoff: Duration::from_millis(250),
            max_backoff: Duration::from_secs(30),
            max_attempts: 16,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WsFrame<Payload = Vec<u8>, Text = String> {
    Text(Text),
    Binary(Payload),
}
