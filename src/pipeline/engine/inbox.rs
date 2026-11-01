use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::engine::{EngineIn, EngineInbox as EngineInboxContract};
use crate::message::ActorMessage;
use crate::queue::{ActorReceiver, ActorRecvResult};

const MESSAGES_PER_SOURCE: usize = 32;

enum Source {
    Binance,
    Aster,
}

pub struct EngineInbox<BINANCE, ASTER> {
    binance_rx: BINANCE,
    aster_rx: ASTER,
    cancellation: Arc<AtomicBool>,
    source: Source,
    remaining: usize,
    binance_closed: bool,
    aster_closed: bool,
}

impl<BINANCE, ASTER> EngineInbox<BINANCE, ASTER> {
    pub const fn new(binance_rx: BINANCE, aster_rx: ASTER, cancellation: Arc<AtomicBool>) -> Self {
        Self {
            binance_rx,
            aster_rx,
            cancellation,
            source: Source::Binance,
            remaining: MESSAGES_PER_SOURCE,
            binance_closed: false,
            aster_closed: false,
        }
    }

    fn next_source(&mut self) {
        self.source = match self.source {
            Source::Binance => Source::Aster,
            Source::Aster => Source::Binance,
        };
        self.remaining = MESSAGES_PER_SOURCE;
    }
}

impl<BINANCE, ASTER> EngineInboxContract for EngineInbox<BINANCE, ASTER>
where
    BINANCE: ActorReceiver<ActorMessage<EngineIn>>,
    ASTER: ActorReceiver<ActorMessage<EngineIn>>,
{
    fn try_recv(&mut self) -> ActorRecvResult<ActorMessage<EngineIn>> {
        for _ in 0..2 {
            if self.cancellation.load(Ordering::Relaxed) {
                return ActorRecvResult::Closed;
            }
            let received = match self.source {
                Source::Binance if self.binance_closed => {
                    self.next_source();
                    continue;
                }
                Source::Aster if self.aster_closed => {
                    self.next_source();
                    continue;
                }
                Source::Binance => self.binance_rx.try_recv(),
                Source::Aster => self.aster_rx.try_recv(),
            };
            match received {
                ActorRecvResult::Message(message) => {
                    self.remaining -= 1;
                    if self.remaining == 0 {
                        self.next_source();
                    }
                    return ActorRecvResult::Message(message);
                }
                ActorRecvResult::Empty => self.next_source(),
                ActorRecvResult::Closed => {
                    match self.source {
                        Source::Binance => self.binance_closed = true,
                        Source::Aster => self.aster_closed = true,
                    }
                    self.next_source();
                }
            }
        }
        if self.binance_closed && self.aster_closed {
            ActorRecvResult::Closed
        } else {
            ActorRecvResult::Empty
        }
    }
}
