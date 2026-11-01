use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bytes::Bytes;
use rustls::ClientConfig;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Connector, Error as WebSocketError, HandshakeError, Message, WebSocket};

use crate::client::tls_config;
use crate::connector::WsFrame;

use super::super::AsterDiffConfig;
use super::{AsterDiffWs, AsterDiffWsClientError};

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

const CONTROL_CHECK_INTERVAL: usize = 4096;

pub struct AsterDiffWsClient {
    tls: Arc<ClientConfig>,
    socket: Option<Socket>,
    stream_url: String,
    stream_host: String,
    stream_port: u16,
    connect_timeout: Duration,
    read_wait: WaitState,
    close_timeout: Duration,
    reconnect_initial_backoff: Duration,
    reconnect_max_backoff: Duration,
    max_reconnect_attempts: u32,
    retry_not_before: Option<Instant>,
}

impl Default for AsterDiffWsClient {
    fn default() -> Self {
        Self::new(&AsterDiffConfig::default())
    }
}

impl AsterDiffWsClient {
    pub fn new(config: &AsterDiffConfig) -> Self {
        Self {
            tls: tls_config(),
            socket: None,
            stream_url: config.stream_url().to_owned(),
            stream_host: config.stream_host().to_owned(),
            stream_port: config.stream_port(),
            connect_timeout: config.connect_timeout(),
            read_wait: WaitState::new(config.read_timeout()),
            close_timeout: config.close_timeout(),
            reconnect_initial_backoff: config.reconnect_initial_backoff(),
            reconnect_max_backoff: config.reconnect_max_backoff(),
            max_reconnect_attempts: config.max_reconnect_attempts(),
            retry_not_before: None,
        }
    }

    fn connect_once(&self, cancellation: &AtomicBool) -> Result<Socket, AsterDiffWsClientError> {
        let mut wait = WaitState::new(self.connect_timeout);
        let tcp = self.connect_tcp(&mut wait, cancellation)?;
        tcp.set_nodelay(true)
            .and_then(|()| tcp.set_nonblocking(true))
            .map_err(AsterDiffWsClientError::Io)?;
        wait.check(cancellation)?;
        let mut handshake = tungstenite::client_tls_with_config(self.stream_url.as_str(), tcp, None, Some(Connector::Rustls(Arc::clone(&self.tls))));
        loop {
            match handshake {
                Ok((socket, _response)) => return Ok(socket),
                Err(HandshakeError::Interrupted(pending)) => {
                    wait.on_pending(cancellation)?;
                    handshake = pending.handshake();
                }
                Err(HandshakeError::Failure(error)) => return Err(error.into()),
            }
        }
    }

    fn connect_tcp(&self, wait: &mut WaitState, cancellation: &AtomicBool) -> Result<TcpStream, AsterDiffWsClientError> {
        wait.check(cancellation)?;
        let mut addresses = (self.stream_host.as_str(), self.stream_port)
            .to_socket_addrs()
            .map_err(AsterDiffWsClientError::Io)?;
        let Some(address) = addresses.next() else {
            return Err(AsterDiffWsClientError::NoResolvedAddress);
        };
        TcpStream::connect_timeout(&address, wait.remaining(cancellation)?).map_err(AsterDiffWsClientError::Io)
    }

    fn wait_for_retry(&mut self, cancellation: &AtomicBool) -> Result<(), AsterDiffWsClientError> {
        let Some(not_before) = self.retry_not_before else {
            return Ok(());
        };
        loop {
            if cancellation.load(Ordering::Relaxed) {
                return Err(AsterDiffWsClientError::Cancelled);
            }
            let remaining = not_before.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                self.retry_not_before = None;
                return Ok(());
            }
        }
    }

    fn is_pending(error: &WebSocketError) -> bool {
        matches!(error, WebSocketError::Io(error) if matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
        ))
    }

    fn read_frame(
        socket: &mut Socket,
        wait: &mut WaitState,
        cancellation: &AtomicBool,
    ) -> Result<Option<WsFrame<Bytes, tungstenite::Utf8Bytes>>, AsterDiffWsClientError> {
        match socket.read() {
            Ok(Message::Text(text)) => Ok(Some(WsFrame::Text(text))),
            Ok(Message::Binary(bytes)) => Ok(Some(WsFrame::Binary(bytes))),
            Ok(Message::Ping(_) | Message::Pong(_)) => {
                wait.check(cancellation)?;
                Ok(None)
            }
            Ok(Message::Close(_)) => {
                match socket.flush() {
                    Ok(()) | Err(WebSocketError::ConnectionClosed) => {}
                    Err(error) if Self::is_pending(&error) => {}
                    Err(error) => return Err(error.into()),
                }
                Err(AsterDiffWsClientError::Closed)
            }
            Ok(Message::Frame(_)) => Err(AsterDiffWsClientError::UnexpectedFrame),
            Err(error) if Self::is_pending(&error) => {
                wait.on_pending(cancellation)?;
                Ok(None)
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl AsterDiffWs for AsterDiffWsClient {
    type Error = AsterDiffWsClientError;

    fn connect(&mut self, cancellation: &AtomicBool) -> Result<(), Self::Error> {
        self.socket = None;
        self.wait_for_retry(cancellation)?;
        let mut backoff = self.reconnect_initial_backoff;
        let mut attempts = 0_u32;
        loop {
            attempts += 1;
            let error = match self.connect_once(cancellation) {
                Ok(socket) => {
                    self.socket = Some(socket);
                    self.read_wait.reset();
                    return Ok(());
                }
                Err(error) => error,
            };
            let Some(minimum_delay) = error.retry_after() else {
                return Err(error);
            };
            let delay = backoff.max(minimum_delay);
            self.retry_not_before = Some(
                Instant::now()
                    .checked_add(delay)
                    .ok_or(AsterDiffWsClientError::InvalidRetryConfig)?,
            );
            if attempts == self.max_reconnect_attempts {
                return Err(error);
            }
            tracing::warn!(
                attempts,
                max_attempts = self.max_reconnect_attempts,
                url = self.stream_url,
                delay = ?delay,
                error = ?error,
                "Aster WebSocket connect failed; retry scheduled",
            );
            self.wait_for_retry(cancellation)?;
            backoff = backoff.saturating_mul(2).min(self.reconnect_max_backoff);
        }
    }

    fn try_recv_frame(&mut self, cancellation: &AtomicBool) -> Result<Option<WsFrame<Bytes, tungstenite::Utf8Bytes>>, Self::Error> {
        let result = match self.socket.as_mut() {
            Some(socket) => Self::read_frame(socket, &mut self.read_wait, cancellation),
            None => Err(AsterDiffWsClientError::NotConnected),
        };
        match result {
            Ok(Some(frame)) => {
                self.read_wait.reset();
                Ok(Some(frame))
            }
            Ok(None) => Ok(None),
            Err(error) => {
                self.read_wait.reset();
                self.socket = None;
                Err(error)
            }
        }
    }

    fn reset_read_timeout(&mut self) {
        self.read_wait.reset();
    }

    fn close(&mut self) -> Result<(), Self::Error> {
        let Some(mut socket) = self.socket.take() else {
            return Ok(());
        };
        let cancellation = AtomicBool::new(false);
        let mut wait = WaitState::new(self.close_timeout);
        wait.check(&cancellation)?;
        let mut result = socket.close(None);
        loop {
            match result {
                Ok(()) | Err(WebSocketError::ConnectionClosed | WebSocketError::AlreadyClosed) => {
                    return Ok(());
                }
                Err(error) if Self::is_pending(&error) => wait.on_pending(&cancellation)?,
                Err(error) => return Err(error.into()),
            }
            result = socket.flush();
        }
    }
}

struct WaitState {
    timeout: Duration,
    expires_at: Option<Instant>,
    pending_polls: usize,
}

impl WaitState {
    fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            expires_at: None,
            pending_polls: 0,
        }
    }

    fn reset(&mut self) {
        self.expires_at = None;
        self.pending_polls = 0;
    }

    fn check(&mut self, cancellation: &AtomicBool) -> Result<(), AsterDiffWsClientError> {
        self.remaining(cancellation).map(|_remaining| ())
    }

    fn remaining(&mut self, cancellation: &AtomicBool) -> Result<Duration, AsterDiffWsClientError> {
        if cancellation.load(Ordering::Relaxed) {
            return Err(AsterDiffWsClientError::Cancelled);
        }
        let now = Instant::now();
        let expires_at = match self.expires_at {
            Some(expires_at) => expires_at,
            None => {
                let expires_at = now
                    .checked_add(self.timeout)
                    .ok_or(AsterDiffWsClientError::InvalidRetryConfig)?;
                self.expires_at = Some(expires_at);
                expires_at
            }
        };
        let remaining = expires_at.saturating_duration_since(now);
        if remaining.is_zero() {
            return Err(AsterDiffWsClientError::Timeout { after: self.timeout });
        }
        Ok(remaining)
    }

    fn on_pending(&mut self, cancellation: &AtomicBool) -> Result<(), AsterDiffWsClientError> {
        if self.pending_polls.is_multiple_of(CONTROL_CHECK_INTERVAL) {
            self.check(cancellation)?;
        }
        self.pending_polls = self.pending_polls.wrapping_add(1);
        Ok(())
    }
}
