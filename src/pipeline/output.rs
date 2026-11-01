use std::sync::atomic::{AtomicBool, Ordering};

use crate::engine::strategy::Signal;
use crate::message::EngineMessage;
use crate::queue::{ActorReceiver, ActorRecvResult};

pub(super) fn run_signal_output<INBOX>(mut inbox: INBOX, cancellation: &AtomicBool)
where
    INBOX: ActorReceiver<EngineMessage<Signal>>,
{
    while !cancellation.load(Ordering::Relaxed) {
        let message = match inbox.try_recv() {
            ActorRecvResult::Message(message) => message,
            ActorRecvResult::Empty => continue,
            ActorRecvResult::Closed => return,
        };
        match &message.actor.payload {
            Signal::CrossVenueSpread(signal) => {
                tracing::info!(
                    engine_seq = message.engine_seq,
                    engine_ts_ns = message.engine_ts_ns,
                    bias = ?signal.bias,
                    reason = ?signal.reason,
                    threshold_bps = signal.threshold_bps,
                    long_bps_x100 = ?signal.long_bps_x100,
                    short_bps_x100 = ?signal.short_bps_x100,
                    binance_bid_e8 = ?signal.binance.bid.map(|price| price.raw()),
                    binance_ask_e8 = ?signal.binance.ask.map(|price| price.raw()),
                    binance_age_ns = ?signal.binance.age_ns,
                    aster_bid_e8 = ?signal.aster.bid.map(|price| price.raw()),
                    aster_ask_e8 = ?signal.aster.ask.map(|price| price.raw()),
                    aster_age_ns = ?signal.aster.age_ns,
                    "strategy signal",
                );
            }
        }
    }
}
