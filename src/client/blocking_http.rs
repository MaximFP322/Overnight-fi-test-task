use std::time::{Duration, Instant};

use ureq::Agent;

use crate::clock::Clock;

use super::HttpClientConfig;

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub struct HttpResponse {
    pub body: Vec<u8>,
    pub received_ts_ns: u64,
}

pub struct BlockingHttpClient {
    agent: Agent,
}

#[derive(Debug)]
pub enum BlockingHttpClientError {
    InvalidTimeout,
    Request(ureq::Error),
    HttpStatus { status: u16, retry_after: Option<Duration> },
}

impl BlockingHttpClient {
    pub fn new(config: HttpClientConfig) -> Result<Self, BlockingHttpClientError> {
        Self::validate(&config)?;
        let agent = Agent::config_builder()
            .timeout_global(Some(config.request_timeout))
            .http_status_as_error(false)
            .https_only(true)
            .max_redirects(0)
            .build()
            .new_agent();
        Ok(Self { agent })
    }

    fn validate(config: &HttpClientConfig) -> Result<(), BlockingHttpClientError> {
        if config.request_timeout.is_zero() || Instant::now().checked_add(config.request_timeout).is_none() {
            return Err(BlockingHttpClientError::InvalidTimeout);
        }
        Ok(())
    }

    pub fn get<CLOCK: Clock>(&mut self, url: &str, clock: &mut CLOCK) -> Result<HttpResponse, BlockingHttpClientError> {
        let mut response = self
            .agent
            .get(url)
            .call()
            .map_err(BlockingHttpClientError::Request)?;
        if !response.status().is_success() {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|header| header.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .map(Duration::from_secs);
            return Err(BlockingHttpClientError::HttpStatus {
                status: response.status().as_u16(),
                retry_after,
            });
        }
        let body = response
            .body_mut()
            .with_config()
            .limit(MAX_RESPONSE_BYTES as u64)
            .read_to_vec()
            .map_err(BlockingHttpClientError::Request)?;
        Ok(HttpResponse {
            body,
            received_ts_ns: clock.now_ns(),
        })
    }
}
