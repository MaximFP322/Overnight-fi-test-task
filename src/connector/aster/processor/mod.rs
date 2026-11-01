mod io;
mod market;
mod reconnect;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::connector::ConnectorIn;

use super::{AsterOutput, AsterSessionError};

pub use market::AsterMarketProcessor;

type ProcessResult<T, W, R> = Result<T, AsterSessionError<W, R>>;

pub trait AsterProcessor {
    type Error;

    fn process_message(&mut self, message: ConnectorIn, outgoing: &mut AsterOutput) -> Result<(), Self::Error>;

    fn step(&mut self, cancellation: &Arc<AtomicBool>, outgoing: &mut AsterOutput) -> Result<(), Self::Error>;

    fn shutdown(&mut self) -> Result<(), Self::Error>;
}
