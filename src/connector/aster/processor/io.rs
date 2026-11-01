use std::sync::atomic::AtomicBool;

use crate::clock::Clock;
use crate::connector::WsFrame;
use crate::message::{ActorMessage, ActorMeta, ActorSequences, ActorTimestamps};

use super::super::parser::{parse_delta, parse_delta_str};
use super::super::{AsterDepthDelta, AsterDiffActorError, AsterDiffRest, AsterDiffWs, AsterSessionError};
use super::{AsterMarketProcessor, ProcessResult};

impl<WS, REST, CLOCK> AsterMarketProcessor<WS, REST, CLOCK>
where
    WS: AsterDiffWs,
    WS::Error: std::fmt::Debug,
    REST: AsterDiffRest,
    REST::Error: std::fmt::Debug,
    CLOCK: Clock,
{
    pub(super) fn try_next_delta(
        &mut self,
        cancellation: &AtomicBool,
    ) -> ProcessResult<Option<ActorMessage<AsterDepthDelta>>, WS::Error, REST::Error> {
        let Some(frame) = self
            .ws
            .try_recv_frame(cancellation)
            .map_err(AsterSessionError::Ws)?
        else {
            return Ok(None);
        };
        let received_ts_ns = self.clock.now_ns();
        let delta = match &frame {
            WsFrame::Text(text) => parse_delta_str(text.as_str(), self.config.aster_symbol(), self.config.instrument_id()),
            WsFrame::Binary(bytes) => parse_delta(bytes, self.config.aster_symbol(), self.config.instrument_id()),
        }
        .map_err(AsterSessionError::Parse)?;
        let actor_ts_ns = self.clock.now_ns();
        Ok(Some(ActorMessage {
            meta: ActorMeta {
                ts: ActorTimestamps {
                    exchange_ts_ns: Some(delta.event_ts_ns),
                    source_ts_ns: Some(delta.transaction_ts_ns),
                    received_ts_ns: Some(received_ts_ns),
                    actor_ts_ns: Some(actor_ts_ns),
                },
                seq: ActorSequences::default(),
            },
            payload: delta,
        }))
    }

    pub(super) fn close(&mut self) -> Result<(), AsterDiffActorError<WS::Error, REST::Error>> {
        self.ws
            .close()
            .map_err(|error| AsterDiffActorError::Session(AsterSessionError::Ws(error)))
    }
}
