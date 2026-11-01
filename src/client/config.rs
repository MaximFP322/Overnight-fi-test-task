use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HttpClientConfig {
    pub request_timeout: Duration,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(10),
        }
    }
}
