use std::convert::Infallible;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::engine::EngineActorError;
use crate::telemetry::TelemetryActorError;

use super::{ActorKind, ConnectorTaskError, PipelineError};

type TelemetryError = TelemetryActorError<Infallible, std::io::Error>;

pub struct Supervisor<B: Debug, A: Debug, E: Debug> {
    cancellation: Arc<AtomicBool>,
    telemetry_producers_done: Arc<AtomicBool>,
    binance: Option<Worker<ConnectorTaskError<B>>>,
    aster: Option<Worker<ConnectorTaskError<A>>>,
    engine: Option<Worker<EngineActorError<E>>>,
    signal_output: Option<Worker<Infallible>>,
    telemetry: Option<Worker<TelemetryError>>,
}

impl<B: Debug, A: Debug, E: Debug> Supervisor<B, A, E> {
    pub fn new(cancellation: Arc<AtomicBool>) -> Self {
        Self {
            cancellation,
            telemetry_producers_done: Arc::new(AtomicBool::new(false)),
            binance: None,
            aster: None,
            engine: None,
            signal_output: None,
            telemetry: None,
        }
    }

    pub fn cancellation(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancellation)
    }

    pub fn start_binance(
        &mut self,
        task: impl FnOnce(Arc<AtomicBool>) -> Result<(), ConnectorTaskError<B>> + Send + 'static,
    ) -> Result<(), PipelineError<B, A, E>>
    where
        B: Send + 'static,
    {
        self.binance = Some(self.spawn(ActorKind::BinanceConnector, task)?);
        Ok(())
    }

    pub fn start_aster(
        &mut self,
        task: impl FnOnce(Arc<AtomicBool>) -> Result<(), ConnectorTaskError<A>> + Send + 'static,
    ) -> Result<(), PipelineError<B, A, E>>
    where
        A: Send + 'static,
    {
        self.aster = Some(self.spawn(ActorKind::AsterConnector, task)?);
        Ok(())
    }

    pub fn start_engine(
        &mut self,
        task: impl FnOnce(Arc<AtomicBool>) -> Result<(), EngineActorError<E>> + Send + 'static,
    ) -> Result<(), PipelineError<B, A, E>>
    where
        E: Send + 'static,
    {
        self.engine = Some(self.spawn(ActorKind::Engine, task)?);
        Ok(())
    }

    pub fn start_signal_output(
        &mut self,
        task: impl FnOnce(Arc<AtomicBool>) -> Result<(), Infallible> + Send + 'static,
    ) -> Result<(), PipelineError<B, A, E>> {
        self.signal_output = Some(self.spawn(ActorKind::SignalOutput, task)?);
        Ok(())
    }

    pub fn start_telemetry(
        &mut self,
        task: impl FnOnce(Arc<AtomicBool>) -> Result<(), TelemetryError> + Send + 'static,
    ) -> Result<(), PipelineError<B, A, E>> {
        self.telemetry = Some(self.spawn(ActorKind::Telemetry, task)?);
        Ok(())
    }

    pub fn wait(mut self) -> Result<(), PipelineError<B, A, E>> {
        let mut result = loop {
            if self.cancellation.load(Ordering::Relaxed) {
                break Ok(());
            }
            if let Some(actor) = self.finished_actor() {
                break if self.cancellation.load(Ordering::Relaxed) {
                    Ok(())
                } else {
                    Err(PipelineError::UnexpectedExit(actor))
                };
            }
            std::thread::sleep(Duration::from_millis(100));
        };

        self.cancellation.store(true, Ordering::Relaxed);
        tracing::info!("stopping actors; draining telemetry after engine exit");
        for error in self.join_workers().into_iter().flatten() {
            if result.is_ok() || matches!(result, Err(PipelineError::UnexpectedExit(_))) {
                result = Err(error);
            } else {
                tracing::error!(?error, "additional actor failure during shutdown");
            }
        }
        tracing::info!("actor pipeline stopped");
        result
    }

    fn spawn<T: Debug + Send + 'static>(
        &self,
        actor: ActorKind,
        task: impl FnOnce(Arc<AtomicBool>) -> Result<(), T> + Send + 'static,
    ) -> Result<Worker<T>, PipelineError<B, A, E>> {
        let task_stop = match actor {
            ActorKind::Telemetry => Arc::clone(&self.telemetry_producers_done),
            ActorKind::BinanceConnector | ActorKind::AsterConnector | ActorKind::Engine | ActorKind::SignalOutput => self.cancellation(),
        };
        Worker::spawn(actor, self.cancellation(), task_stop, task).map_err(|source| PipelineError::ThreadStart { actor, source })
    }

    fn finished_actor(&self) -> Option<ActorKind> {
        [
            (ActorKind::BinanceConnector, self.binance.as_ref().is_some_and(Worker::is_finished)),
            (ActorKind::AsterConnector, self.aster.as_ref().is_some_and(Worker::is_finished)),
            (ActorKind::Engine, self.engine.as_ref().is_some_and(Worker::is_finished)),
            (ActorKind::SignalOutput, self.signal_output.as_ref().is_some_and(Worker::is_finished)),
            (ActorKind::Telemetry, self.telemetry.as_ref().is_some_and(Worker::is_finished)),
        ]
        .into_iter()
        .find_map(|(actor, finished)| finished.then_some(actor))
    }

    fn join_workers(&mut self) -> [Option<PipelineError<B, A, E>>; 5] {
        let engine = join_worker(self.engine.take(), PipelineError::Engine);
        self.telemetry_producers_done.store(true, Ordering::Release);
        [
            join_worker(self.binance.take(), |error| PipelineError::BinanceConnector(Box::new(error))),
            join_worker(self.aster.take(), |error| PipelineError::AsterConnector(Box::new(error))),
            engine,
            join_worker(self.signal_output.take(), |error| match error {}),
            join_worker(self.telemetry.take(), PipelineError::Telemetry),
        ]
    }
}

impl<B: Debug, A: Debug, E: Debug> Drop for Supervisor<B, A, E> {
    fn drop(&mut self) {
        self.cancellation.store(true, Ordering::Relaxed);
        for error in self.join_workers().into_iter().flatten() {
            tracing::error!(?error, "actor failure during pipeline cleanup");
        }
    }
}

fn join_worker<T: Debug, B, A, E>(worker: Option<Worker<T>>, wrap: impl FnOnce(T) -> PipelineError<B, A, E>) -> Option<PipelineError<B, A, E>> {
    let mut worker = worker?;
    match worker.join() {
        Ok(()) => None,
        Err(WorkerError::Run(error)) => Some(wrap(error)),
        Err(WorkerError::Panicked) => Some(PipelineError::ThreadPanicked { actor: worker.actor }),
    }
}

struct Worker<E: Debug> {
    actor: ActorKind,
    cancellation: Arc<AtomicBool>,
    task_stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<(), E>>>,
}

impl<E: Debug> Worker<E> {
    fn spawn(
        actor: ActorKind,
        cancellation: Arc<AtomicBool>,
        task_stop: Arc<AtomicBool>,
        task: impl FnOnce(Arc<AtomicBool>) -> Result<(), E> + Send + 'static,
    ) -> std::io::Result<Self>
    where
        E: Send + 'static,
    {
        let worker_cancellation = Arc::clone(&cancellation);
        let worker_stop = Arc::clone(&task_stop);
        let thread = std::thread::Builder::new()
            .name(thread_name(actor).to_owned())
            .spawn(move || {
                let result = task(worker_stop);
                if result.is_err() {
                    worker_cancellation.store(true, Ordering::Relaxed);
                }
                result
            })?;
        Ok(Self {
            actor,
            cancellation,
            task_stop,
            thread: Some(thread),
        })
    }

    fn is_finished(&self) -> bool {
        self.thread.as_ref().is_some_and(JoinHandle::is_finished)
    }

    fn join(&mut self) -> Result<(), WorkerError<E>> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        match thread.join() {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => Err(WorkerError::Run(error)),
            Err(_panic) => Err(WorkerError::Panicked),
        }
    }
}

impl<E: Debug> Drop for Worker<E> {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.cancellation.store(true, Ordering::Relaxed);
            self.task_stop.store(true, Ordering::Release);
            if let Err(error) = self.join() {
                tracing::error!(actor = ?self.actor, ?error, "actor failure during worker cleanup");
            }
        }
    }
}

#[derive(Debug)]
enum WorkerError<E> {
    Run(E),
    Panicked,
}

fn thread_name(actor: ActorKind) -> &'static str {
    match actor {
        ActorKind::BinanceConnector => "binance",
        ActorKind::AsterConnector => "aster",
        ActorKind::Engine => "engine",
        ActorKind::SignalOutput => "signal-output",
        ActorKind::Telemetry => "telemetry",
    }
}
