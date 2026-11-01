use std::io::ErrorKind;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use backon::BackoffBuilder;

use crate::client::{BlockingHttpClient, BlockingHttpClientError, HttpResponse};
use crate::clock::Clock;
use crate::connector::RestRetryConfig;
use crate::instrument::InstrumentId;

use super::super::AsterDiffConfig;
use super::super::parser::parse_snapshot;
use super::{AsterDiffRest, AsterRestError, ReceivedAsterSnapshot};

pub struct AsterRest<CLOCK> {
    client: BlockingHttpClient,
    clock: CLOCK,
    retry: RestRetryConfig,
    instrument_id: InstrumentId,
    snapshot_url: String,
    server_retry_at: Option<Instant>,
}

impl<CLOCK> AsterRest<CLOCK> {
    pub fn new(client: BlockingHttpClient, clock: CLOCK, retry: RestRetryConfig, config: &AsterDiffConfig) -> Self {
        let snapshot_url = format!(
            "{}?symbol={}&limit={}",
            config.snapshot_url(),
            config.aster_symbol().as_str().to_ascii_uppercase(),
            config.snapshot_limit(),
        );
        Self {
            client,
            clock,
            retry,
            instrument_id: config.instrument_id(),
            snapshot_url,
            server_retry_at: None,
        }
    }

    fn is_retryable_error(error: &AsterRestError) -> bool {
        match error {
            AsterRestError::Http(error) => Self::is_retryable_http_error(error),
            AsterRestError::Parse(_) | AsterRestError::InvalidRetryConfig | AsterRestError::Cancelled => false,
        }
    }

    fn is_retryable_http_error(error: &BlockingHttpClientError) -> bool {
        match error {
            BlockingHttpClientError::InvalidTimeout => false,
            BlockingHttpClientError::HttpStatus { status, .. } => Self::is_retryable_status(*status),
            BlockingHttpClientError::Request(error) => Self::is_retryable_request_error(error),
        }
    }

    fn is_retryable_status(status: u16) -> bool {
        matches!(status, 418 | 429 | 500..=599)
    }

    fn retry_after(error: &AsterRestError) -> Duration {
        let AsterRestError::Http(BlockingHttpClientError::HttpStatus { status, retry_after }) = error else {
            return Duration::ZERO;
        };
        retry_after.unwrap_or_else(|| match status {
            418 => Duration::from_secs(120),
            429 => Duration::from_secs(60),
            _ => Duration::ZERO,
        })
    }

    fn is_retryable_request_error(error: &ureq::Error) -> bool {
        match error {
            ureq::Error::Io(error) => Self::is_retryable_io_error(error),
            ureq::Error::Timeout(_) | ureq::Error::HostNotFound | ureq::Error::ConnectionFailed => true,
            _ => false,
        }
    }

    fn is_retryable_io_error(error: &std::io::Error) -> bool {
        !matches!(
            error.kind(),
            ErrorKind::InvalidInput | ErrorKind::InvalidData | ErrorKind::PermissionDenied | ErrorKind::Unsupported
        )
    }
}

impl<CLOCK: Clock> AsterRest<CLOCK> {
    fn get_with_retry(&mut self, cancellation: &Arc<AtomicBool>) -> Result<HttpResponse, AsterRestError> {
        self.wait_for_server_retry(cancellation)?;
        let mut delays = self.retry.backoff().build();
        let max_attempts = self.retry.max_attempts();
        let mut attempts = 0_u32;
        loop {
            attempts += 1;
            let error = match self.get_once(cancellation) {
                Ok(response) => return Ok(response),
                Err(error) => error,
            };
            if !Self::is_retryable_error(&error) {
                return Err(error);
            }
            let server_delay = Self::retry_after(&error);
            self.server_retry_at = Some(
                Instant::now()
                    .checked_add(server_delay)
                    .ok_or(AsterRestError::InvalidRetryConfig)?,
            );
            let Some(backoff) = delays.next() else {
                return Err(error);
            };
            let delay = backoff.max(server_delay);
            tracing::warn!(attempts, max_attempts, delay = ?delay, error = ?error,
                "Aster snapshot request failed; retry scheduled");
            Self::wait_for(delay, cancellation)?;
            self.server_retry_at = None;
        }
    }

    fn get_once(&mut self, cancellation: &AtomicBool) -> Result<HttpResponse, AsterRestError> {
        Self::check_cancellation(cancellation)?;
        self.client
            .get(&self.snapshot_url, &mut self.clock)
            .map_err(AsterRestError::Http)
    }

    fn check_cancellation(cancellation: &AtomicBool) -> Result<(), AsterRestError> {
        if cancellation.load(Ordering::Relaxed) {
            return Err(AsterRestError::Cancelled);
        }
        Ok(())
    }

    fn wait_for_server_retry(&mut self, cancellation: &AtomicBool) -> Result<(), AsterRestError> {
        if let Some(retry_at) = self.server_retry_at {
            Self::wait_for(retry_at.saturating_duration_since(Instant::now()), cancellation)?;
            self.server_retry_at = None;
        }
        Ok(())
    }

    fn wait_for(delay: Duration, cancellation: &AtomicBool) -> Result<(), AsterRestError> {
        let started = Instant::now();
        loop {
            Self::check_cancellation(cancellation)?;
            let remaining = delay.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Ok(());
            }
        }
    }
}

impl<CLOCK: Clock> AsterDiffRest for AsterRest<CLOCK> {
    type Error = AsterRestError;

    fn fetch_snapshot(&mut self, cancellation: &Arc<AtomicBool>) -> Result<ReceivedAsterSnapshot, Self::Error> {
        let response = self.get_with_retry(cancellation)?;
        let snapshot = parse_snapshot(&response.body, self.instrument_id).map_err(AsterRestError::Parse)?;
        let parsed_ts_ns = self.clock.now_ns();
        Ok(ReceivedAsterSnapshot {
            received_ts_ns: response.received_ts_ns,
            parsed_ts_ns,
            snapshot,
        })
    }
}
