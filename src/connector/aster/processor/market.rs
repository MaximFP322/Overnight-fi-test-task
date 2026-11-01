use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crate::clock::Clock;
use crate::connector::ConnectorIn;

use super::super::{AsterAction, AsterDiffActorError, AsterDiffConfig, AsterDiffRest, AsterDiffWs, AsterEvent, AsterSessionError, AsterState};
use super::{AsterOutput, AsterProcessor};

#[derive(Clone, Copy)]
pub(super) enum Phase {
    Start,
    Connecting,
    FetchSnapshot,
    Reading,
    Recover,
    Backoff(Instant),
    Stopped,
}

pub struct AsterMarketProcessor<WS: AsterDiffWs, REST: AsterDiffRest, CLOCK> {
    pub(super) config: AsterDiffConfig,
    pub(super) state: AsterState,
    pub(super) ws: WS,
    pub(super) rest: REST,
    pub(super) clock: CLOCK,
    pub(super) phase: Phase,
    pub(super) pending_error: Option<AsterSessionError<WS::Error, REST::Error>>,
    pub(super) reconnect_attempts: u32,
    pub(super) backoff: Duration,
    pub(super) live_since: Option<Instant>,
}

impl<WS: AsterDiffWs, REST: AsterDiffRest, CLOCK> AsterMarketProcessor<WS, REST, CLOCK> {
    pub fn new(config: AsterDiffConfig, ws: WS, rest: REST, clock: CLOCK) -> Self {
        let state = AsterState::new(config.instrument_id());
        let backoff = config.reconnect_initial_backoff();
        Self {
            config,
            state,
            ws,
            rest,
            clock,
            phase: Phase::Start,
            pending_error: None,
            reconnect_attempts: 0,
            backoff,
            live_since: None,
        }
    }
}

impl<WS, REST, CLOCK> AsterMarketProcessor<WS, REST, CLOCK>
where
    WS: AsterDiffWs,
    WS::Error: std::fmt::Debug,
    REST: AsterDiffRest,
    REST::Error: std::fmt::Debug,
    CLOCK: Clock,
{
    pub(super) fn apply_event(&mut self, event: AsterEvent, outgoing: &mut AsterOutput) -> Result<(), AsterSessionError<WS::Error, REST::Error>> {
        match self
            .state
            .process_message(event, self.clock.now_ns(), outgoing)?
        {
            AsterAction::None => {}
            AsterAction::FetchSnapshot => {
                self.live_since = None;
                self.phase = Phase::FetchSnapshot;
            }
            AsterAction::Resync { reason } => {
                tracing::warn!(?reason, "Aster stream requires resync");
                self.live_since = None;
                self.phase = Phase::FetchSnapshot;
            }
            AsterAction::Synchronized { snapshot_id, last_update_id } => {
                self.live_since = Some(Instant::now());
                tracing::info!(instrument_id = ?self.config.instrument_id(),
                    aster_symbol = ?self.config.aster_symbol(), epoch = self.state.epoch(),
                    snapshot_id, last_update_id, "Aster diff feed synchronized");
            }
            AsterAction::IgnoredInstrument { instrument_id } => {
                tracing::warn!(?instrument_id, "Aster resync command for another instrument dropped");
            }
        }
        Ok(())
    }

    fn begin_session(&mut self, outgoing: &mut AsterOutput) -> Result<(), AsterSessionError<WS::Error, REST::Error>> {
        self.live_since = None;
        self.phase = Phase::Connecting;
        self.apply_event(AsterEvent::SessionStarted, outgoing)
    }

    fn process_phase(&mut self, cancellation: &Arc<AtomicBool>, outgoing: &mut AsterOutput) -> Result<(), AsterSessionError<WS::Error, REST::Error>> {
        match self.phase {
            Phase::Start => self.begin_session(outgoing),
            Phase::Connecting => {
                self.ws
                    .connect(cancellation)
                    .map_err(AsterSessionError::Connect)?;
                if cancellation.load(Ordering::Relaxed) {
                    self.phase = Phase::Stopped;
                    return Ok(());
                }
                self.apply_event(AsterEvent::Connected, outgoing)
            }
            Phase::FetchSnapshot => {
                let snapshot = self
                    .rest
                    .fetch_snapshot(cancellation)
                    .map_err(AsterSessionError::Rest)?;
                self.ws.reset_read_timeout();
                if cancellation.load(Ordering::Relaxed) {
                    self.phase = Phase::Stopped;
                    return Ok(());
                }
                self.phase = Phase::Reading;
                self.apply_event(AsterEvent::Snapshot(snapshot), outgoing)
            }
            Phase::Reading => {
                let Some(delta) = self.try_next_delta(cancellation)? else {
                    return Ok(());
                };
                self.apply_event(AsterEvent::Delta(delta), outgoing)
            }
            Phase::Backoff(deadline) => {
                if Instant::now() >= deadline {
                    self.begin_session(outgoing)
                } else {
                    Ok(())
                }
            }
            Phase::Recover | Phase::Stopped => Ok(()),
        }
    }
}

impl<WS, REST, CLOCK> AsterProcessor for AsterMarketProcessor<WS, REST, CLOCK>
where
    WS: AsterDiffWs,
    WS::Error: std::fmt::Debug,
    REST: AsterDiffRest,
    REST::Error: std::fmt::Debug,
    CLOCK: Clock,
{
    type Error = AsterDiffActorError<WS::Error, REST::Error>;

    fn process_message(&mut self, message: ConnectorIn, outgoing: &mut AsterOutput) -> Result<(), Self::Error> {
        if matches!(self.phase, Phase::Recover | Phase::Stopped) {
            return Ok(());
        }
        match self.apply_event(AsterEvent::Command(message), outgoing) {
            Ok(()) => Ok(()),
            Err(error) => self.on_error(error, outgoing),
        }
    }

    fn step(&mut self, cancellation: &Arc<AtomicBool>, outgoing: &mut AsterOutput) -> Result<(), Self::Error> {
        if cancellation.load(Ordering::Relaxed) {
            return Ok(());
        }
        if matches!(self.phase, Phase::Recover) {
            return self.recover();
        }
        match self.process_phase(cancellation, outgoing) {
            Ok(()) => Ok(()),
            Err(_) if cancellation.load(Ordering::Relaxed) => {
                for message in outgoing {
                    *message = None;
                }
                Ok(())
            }
            Err(error) => self.on_error(error, outgoing),
        }
    }

    fn shutdown(&mut self) -> Result<(), Self::Error> {
        self.phase = Phase::Stopped;
        self.close()
    }
}
