use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::connector::aster::{AsterDiffConnectorActor, AsterProcessor};
use crate::connector::binance::{BinanceDiffConnectorActor, BinanceProcessor};
use crate::connector::{ConnectorIn, ConnectorOutbox};
use crate::pipeline::ConnectorTaskError;
use crate::queue::ActorReceiver;

type BinanceTaskResult<E> = Result<(), ConnectorTaskError<E>>;
type AsterTaskResult<E> = Result<(), ConnectorTaskError<E>>;

pub(crate) fn run_binance_until_cancelled<PROCESSOR, INBOX, OUTBOX>(
    mut actor: BinanceDiffConnectorActor<PROCESSOR, INBOX, OUTBOX>,
    cancellation: Arc<AtomicBool>,
) -> BinanceTaskResult<PROCESSOR::Error>
where
    PROCESSOR: BinanceProcessor,
    INBOX: ActorReceiver<ConnectorIn>,
    OUTBOX: ConnectorOutbox,
{
    let result = actor.run(&cancellation);

    if result.is_err() {
        cancellation.store(true, Ordering::Relaxed);
    }

    match (result, actor.shutdown()) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(ConnectorTaskError::Run(error)),
        (Ok(()), Err(error)) => Err(ConnectorTaskError::Shutdown(error)),
        (Err(run), Err(shutdown)) => Err(ConnectorTaskError::RunAndShutdown { run, shutdown }),
    }
}

pub(crate) fn run_aster_until_cancelled<PROCESSOR, INBOX, OUTBOX>(
    mut actor: AsterDiffConnectorActor<PROCESSOR, INBOX, OUTBOX>,
    cancellation: Arc<AtomicBool>,
) -> AsterTaskResult<PROCESSOR::Error>
where
    PROCESSOR: AsterProcessor,
    INBOX: ActorReceiver<ConnectorIn>,
    OUTBOX: ConnectorOutbox,
{
    let result = actor.run(&cancellation);

    if result.is_err() {
        cancellation.store(true, Ordering::Relaxed);
    }

    match (result, actor.shutdown()) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(ConnectorTaskError::Run(error)),
        (Ok(()), Err(error)) => Err(ConnectorTaskError::Shutdown(error)),
        (Err(run), Err(shutdown)) => Err(ConnectorTaskError::RunAndShutdown { run, shutdown }),
    }
}
