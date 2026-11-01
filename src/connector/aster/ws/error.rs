use std::time::Duration;

use tungstenite::Error as WebSocketError;
use tungstenite::error::ProtocolError;

#[derive(Debug)]
pub enum AsterDiffWsClientError {
    Io(std::io::Error),
    WebSocket(WebSocketError),
    NoResolvedAddress,
    NotConnected,
    Closed,
    Cancelled,
    UnexpectedFrame,
    InvalidRetryConfig,
    Timeout { after: Duration },
    HandshakeStatus { status: u16, retry_after: Option<Duration> },
}

impl AsterDiffWsClientError {
    pub(super) fn retry_after(&self) -> Option<Duration> {
        if !self.is_retryable() {
            return None;
        }
        let delay = if let Self::HandshakeStatus { status, retry_after } = self {
            retry_after.unwrap_or_else(|| Self::default_status_retry_delay(*status))
        } else {
            Duration::ZERO
        };
        Some(delay)
    }

    fn is_retryable(&self) -> bool {
        match self {
            Self::HandshakeStatus { status, .. } => {
                matches!(status, 418 | 429 | 500..=599)
            }
            Self::WebSocket(error) => Self::is_retryable_websocket_error(error),
            Self::Io(_) | Self::NoResolvedAddress | Self::NotConnected | Self::Closed | Self::UnexpectedFrame | Self::Timeout { .. } => true,
            Self::Cancelled | Self::InvalidRetryConfig => false,
        }
    }

    fn is_retryable_websocket_error(error: &WebSocketError) -> bool {
        match error {
            WebSocketError::Url(_)
            | WebSocketError::HttpFormat(_)
            | WebSocketError::Tls(_)
            | WebSocketError::Protocol(ProtocolError::SecWebSocketSubProtocolError(_)) => false,
            WebSocketError::Http(response) => {
                matches!(response.status().as_u16(), 418 | 429 | 500..=599)
            }
            WebSocketError::Io(error) => error.kind() != std::io::ErrorKind::InvalidData,
            WebSocketError::ConnectionClosed
            | WebSocketError::AlreadyClosed
            | WebSocketError::Capacity(_)
            | WebSocketError::Protocol(_)
            | WebSocketError::WriteBufferFull(_)
            | WebSocketError::Utf8(_)
            | WebSocketError::AttackAttempt => true,
        }
    }

    fn default_status_retry_delay(status: u16) -> Duration {
        match status {
            418 => Duration::from_secs(120),
            429 => Duration::from_secs(60),
            _ => Duration::ZERO,
        }
    }
}

impl From<WebSocketError> for AsterDiffWsClientError {
    fn from(error: WebSocketError) -> Self {
        match error {
            WebSocketError::Http(response) => Self::HandshakeStatus {
                status: response.status().as_u16(),
                retry_after: response
                    .headers()
                    .get("retry-after")
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.parse::<u64>().ok())
                    .map(Duration::from_secs),
            },
            error => Self::WebSocket(error),
        }
    }
}
