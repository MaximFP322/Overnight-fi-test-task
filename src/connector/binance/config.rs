use std::time::Duration;

use http::Uri;

use crate::instrument::InstrumentId;

use super::BinanceSymbol;

#[derive(Debug, Clone)]
pub struct BinanceDiffConfigInput {
    pub instrument_id: InstrumentId,
    pub binance_symbol: BinanceSymbol,
    pub stream_url: String,
    pub snapshot_url: String,
    pub snapshot_limit: u16,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub close_timeout: Duration,
    pub reconnect_initial_backoff: Duration,
    pub reconnect_max_backoff: Duration,
    pub max_reconnect_attempts: u32,
}

#[derive(Debug, Clone)]
pub struct BinanceDiffConfig {
    input: BinanceDiffConfigInput,
    stream_host: String,
    stream_port: u16,
}

#[derive(Debug)]
pub enum BinanceConfigError {
    InvalidSymbol,
    StreamUri(http::uri::InvalidUri),
    InvalidStreamUrl,
    SnapshotUri(http::uri::InvalidUri),
    InvalidSnapshotUrl,
    InvalidSnapshotLimit(u16),
    ZeroTimeout,
    InvalidBackoff,
    ZeroReconnectAttempts,
}

impl BinanceDiffConfigInput {
    fn validate(&self) -> Result<(String, u16), BinanceConfigError> {
        let symbol = self.binance_symbol.as_str();
        if symbol.is_empty() || !symbol.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
            return Err(BinanceConfigError::InvalidSymbol);
        }
        let uri: Uri = self
            .stream_url
            .parse()
            .map_err(BinanceConfigError::StreamUri)?;
        if uri.scheme_str() != Some("wss")
            || uri
                .authority()
                .is_some_and(|authority| authority.as_str().contains('@'))
        {
            return Err(BinanceConfigError::InvalidStreamUrl);
        }
        let host = uri.host().ok_or(BinanceConfigError::InvalidStreamUrl)?;
        let stream = if uri.query().is_none() {
            uri.path()
                .strip_prefix("/public/ws/")
                .or_else(|| uri.path().strip_prefix("/ws/"))
        } else if matches!(uri.path(), "/public/stream" | "/stream") {
            uri.query().and_then(|query| query.strip_prefix("streams="))
        } else {
            None
        };
        let expected = format!("{}@depth", symbol.to_ascii_lowercase());
        if !stream.is_some_and(|stream| stream == expected || stream == format!("{expected}@100ms") || stream == format!("{expected}@500ms")) {
            return Err(BinanceConfigError::InvalidStreamUrl);
        }
        let snapshot_uri: Uri = self
            .snapshot_url
            .parse()
            .map_err(BinanceConfigError::SnapshotUri)?;
        if self.snapshot_url.contains('#')
            || snapshot_uri.scheme_str() != Some("https")
            || snapshot_uri.host().is_none()
            || snapshot_uri
                .authority()
                .is_some_and(|authority| authority.as_str().contains('@'))
        {
            return Err(BinanceConfigError::InvalidSnapshotUrl);
        }
        if !matches!(self.snapshot_limit, 5 | 10 | 20 | 50 | 100 | 500 | 1000) {
            return Err(BinanceConfigError::InvalidSnapshotLimit(self.snapshot_limit));
        }
        if [self.connect_timeout, self.read_timeout, self.close_timeout]
            .iter()
            .any(Duration::is_zero)
        {
            return Err(BinanceConfigError::ZeroTimeout);
        }
        if self.reconnect_initial_backoff.is_zero() || self.reconnect_max_backoff < self.reconnect_initial_backoff {
            return Err(BinanceConfigError::InvalidBackoff);
        }
        if self.max_reconnect_attempts == 0 {
            return Err(BinanceConfigError::ZeroReconnectAttempts);
        }
        Ok((host.to_owned(), uri.port_u16().unwrap_or(443)))
    }
}

impl BinanceDiffConfig {
    pub fn new(input: BinanceDiffConfigInput) -> Result<Self, BinanceConfigError> {
        let (stream_host, stream_port) = input.validate()?;
        Ok(Self {
            input,
            stream_host,
            stream_port,
        })
    }

    pub const fn instrument_id(&self) -> InstrumentId {
        self.input.instrument_id
    }

    pub fn binance_symbol(&self) -> &BinanceSymbol {
        &self.input.binance_symbol
    }

    pub fn snapshot_url(&self) -> &str {
        &self.input.snapshot_url
    }

    pub const fn snapshot_limit(&self) -> u16 {
        self.input.snapshot_limit
    }

    pub(super) fn stream_url(&self) -> &str {
        &self.input.stream_url
    }

    pub(super) fn stream_host(&self) -> &str {
        &self.stream_host
    }

    pub(super) const fn stream_port(&self) -> u16 {
        self.stream_port
    }

    pub(super) const fn connect_timeout(&self) -> Duration {
        self.input.connect_timeout
    }

    pub(super) const fn read_timeout(&self) -> Duration {
        self.input.read_timeout
    }

    pub(super) const fn close_timeout(&self) -> Duration {
        self.input.close_timeout
    }

    pub(super) const fn reconnect_initial_backoff(&self) -> Duration {
        self.input.reconnect_initial_backoff
    }

    pub(super) const fn reconnect_max_backoff(&self) -> Duration {
        self.input.reconnect_max_backoff
    }

    pub(super) const fn max_reconnect_attempts(&self) -> u32 {
        self.input.max_reconnect_attempts
    }
}

impl TryFrom<BinanceDiffConfigInput> for BinanceDiffConfig {
    type Error = BinanceConfigError;

    fn try_from(input: BinanceDiffConfigInput) -> Result<Self, Self::Error> {
        Self::new(input)
    }
}

impl Default for BinanceDiffConfigInput {
    fn default() -> Self {
        Self {
            instrument_id: InstrumentId(1),
            binance_symbol: BinanceSymbol("BTCUSDT".to_owned()),
            stream_url: "wss://fstream.binance.com/public/ws/btcusdt@depth@100ms".to_owned(),
            snapshot_url: "https://fapi.binance.com/fapi/v1/depth".to_owned(),
            snapshot_limit: 1000,
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            close_timeout: Duration::from_secs(2),
            reconnect_initial_backoff: Duration::from_millis(250),
            reconnect_max_backoff: Duration::from_secs(30),
            max_reconnect_attempts: 16,
        }
    }
}

impl Default for BinanceDiffConfig {
    fn default() -> Self {
        Self {
            input: BinanceDiffConfigInput::default(),
            stream_host: "fstream.binance.com".to_owned(),
            stream_port: 443,
        }
    }
}
