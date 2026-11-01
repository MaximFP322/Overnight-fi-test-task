mod client;
mod error;

pub use client::AsterDiffWsClient;
pub use error::AsterDiffWsClientError;

use std::sync::atomic::AtomicBool;

use bytes::Bytes;

use crate::connector::WsFrame;

pub trait AsterDiffWs {
    type Error;

    fn connect(&mut self, cancellation: &AtomicBool) -> Result<(), Self::Error>;

    fn try_recv_frame(&mut self, cancellation: &AtomicBool) -> Result<Option<WsFrame<Bytes, tungstenite::Utf8Bytes>>, Self::Error>;

    fn reset_read_timeout(&mut self);

    fn close(&mut self) -> Result<(), Self::Error>;
}
