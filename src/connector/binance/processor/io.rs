use std::sync::atomic::AtomicBool;

use crate::clock::Clock;
use crate::connector::WsFrame;
use crate::message::{ActorMessage, ActorMeta, ActorSequences, ActorTimestamps};

use super::super::parser::{parse_delta, parse_delta_str};
use super::super::{BinanceDepthDelta, BinanceDiffActorError, BinanceDiffRest, BinanceDiffWs, BinanceSessionError};
use super::{BinanceMarketProcessor, ProcessResult};

impl<WS, REST, CLOCK> BinanceMarketProcessor<WS, REST, CLOCK>
where
    WS: BinanceDiffWs,
    WS::Error: std::fmt::Debug,
    REST: BinanceDiffRest,
    REST::Error: std::fmt::Debug,
    CLOCK: Clock,
{
    pub(super) fn try_next_delta(
        &mut self,
        cancellation: &AtomicBool,
    ) -> ProcessResult<Option<ActorMessage<BinanceDepthDelta>>, WS::Error, REST::Error> {
        let Some(frame) = self
            .ws
            .try_recv_frame(cancellation)
            .map_err(BinanceSessionError::Ws)?
        else {
            return Ok(None);
        };
        let received_ts_ns = self.clock.now_ns();
        let delta = match &frame {
            WsFrame::Text(text) => parse_delta_str(text.as_str(), self.config.binance_symbol(), self.config.instrument_id()),
            WsFrame::Binary(bytes) => parse_delta(bytes, self.config.binance_symbol(), self.config.instrument_id()),
        }
        .map_err(BinanceSessionError::Parse)?;
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

    pub(super) fn close(&mut self) -> Result<(), BinanceDiffActorError<WS::Error, REST::Error>> {
        self.ws
            .close()
            .map_err(|error| BinanceDiffActorError::Session(BinanceSessionError::Ws(error)))
    }
}
