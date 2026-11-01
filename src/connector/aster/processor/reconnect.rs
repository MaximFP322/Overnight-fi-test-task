use std::time::{Duration, Instant};

use crate::clock::Clock;

use super::super::{AsterConfigError, AsterDiffActorError, AsterDiffRest, AsterDiffWs, AsterEvent, AsterSessionError};
use super::market::Phase;
use super::{AsterMarketProcessor, AsterOutput};

impl<WS, REST, CLOCK> AsterMarketProcessor<WS, REST, CLOCK>
where
    WS: AsterDiffWs,
    WS::Error: std::fmt::Debug,
    REST: AsterDiffRest,
    REST::Error: std::fmt::Debug,
    CLOCK: Clock,
{
    pub(super) fn on_error(
        &mut self,
        error: AsterSessionError<WS::Error, REST::Error>,
        outgoing: &mut AsterOutput,
    ) -> Result<(), AsterDiffActorError<WS::Error, REST::Error>> {
        for message in outgoing.iter_mut() {
            *message = None;
        }
        tracing::warn!(instrument_id = ?self.config.instrument_id(),
            aster_symbol = ?self.config.aster_symbol(), epoch = self.state.epoch(), error = ?error,
            "Aster diff session stopped");
        self.pending_error = Some(error);
        self.phase = Phase::Recover;
        self.apply_event(AsterEvent::Disconnected, outgoing)
            .map_err(AsterDiffActorError::Session)
    }

    pub(super) fn recover(&mut self) -> Result<(), AsterDiffActorError<WS::Error, REST::Error>> {
        self.phase = Phase::Stopped;
        let Some(error) = self.pending_error.take() else {
            return Ok(());
        };
        if let Err(close_error) = self.close() {
            tracing::warn!(error = ?close_error, "Aster session close failed");
        }
        if !error.is_recoverable() {
            return Err(AsterDiffActorError::Session(error));
        }
        if self
            .live_since
            .is_some_and(|since| since.elapsed() >= Duration::from_secs(30))
        {
            self.reconnect_attempts = 0;
            self.backoff = self.config.reconnect_initial_backoff();
        }
        self.reconnect_attempts = self.reconnect_attempts.saturating_add(1);
        if self.reconnect_attempts >= self.config.max_reconnect_attempts() {
            return Err(AsterDiffActorError::ReconnectExhausted {
                attempts: self.reconnect_attempts,
                last: error,
            });
        }
        tracing::info!(attempts = self.reconnect_attempts, delay = ?self.backoff, "Aster reconnect scheduled");
        let deadline = Instant::now()
            .checked_add(self.backoff)
            .ok_or(AsterDiffActorError::Config(AsterConfigError::InvalidBackoff))?;
        self.phase = Phase::Backoff(deadline);
        self.backoff = self
            .backoff
            .saturating_mul(2)
            .min(self.config.reconnect_max_backoff());
        Ok(())
    }
}
