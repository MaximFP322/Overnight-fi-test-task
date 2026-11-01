use crate::connector::{ConnectorIn, ConnectorOut};
use crate::instrument::InstrumentId;
use crate::market::{FeedState, FeedStatus, MarketEvent, Venue};
use crate::message::{ActorMessage, ActorMeta, ActorSequences, ActorTimestamps};

use super::super::BinanceDepthDelta;
use super::sync::{DepthSync, SyncAction};
use super::{BinanceAction, BinanceEvent, BinanceOutput, BinanceStateError, BinanceSyncError};

const MAX_STALE_SNAPSHOTS: u32 = 3;

pub struct BinanceState {
    instrument_id: InstrumentId,
    epoch: u64,
    next_seq: u64,
    sync: Option<DepthSync>,
    snapshot_id: u64,
    stale_snapshots: u32,
}

impl BinanceState {
    pub const fn new(instrument_id: InstrumentId) -> Self {
        Self {
            instrument_id,
            epoch: 0,
            next_seq: 0,
            sync: None,
            snapshot_id: 0,
            stale_snapshots: 0,
        }
    }

    fn start_sync(&mut self, now_ns: u64, outgoing: &mut BinanceOutput) -> Result<BinanceAction, BinanceStateError> {
        self.sync = None;
        outgoing[0] = Some(self.status_message(FeedState::Syncing, now_ns)?);
        Ok(BinanceAction::FetchSnapshot)
    }

    fn apply_delta(
        &mut self,
        delta: ActorMessage<BinanceDepthDelta>,
        now_ns: u64,
        outgoing: &mut BinanceOutput,
    ) -> Result<BinanceAction, BinanceStateError> {
        let sync = self
            .sync
            .as_mut()
            .ok_or(BinanceStateError::Sync(BinanceSyncError::MissingSnapshot))?;
        let action = sync.apply(&delta.payload.book);
        let last_update_id = delta.payload.book.final_update_id;
        match action {
            SyncAction::Ignore => Ok(BinanceAction::None),
            SyncAction::Apply => {
                outgoing[0] = Some(self.market_message(MarketEvent::Delta(delta.payload.book), delta.meta.ts)?);
                Ok(BinanceAction::None)
            }
            SyncAction::Bridge => {
                self.stale_snapshots = 0;
                outgoing[0] = Some(self.market_message(MarketEvent::Delta(delta.payload.book), delta.meta.ts)?);
                outgoing[1] = Some(self.status_message(FeedState::Live, now_ns)?);
                Ok(BinanceAction::Synchronized {
                    snapshot_id: self.snapshot_id,
                    last_update_id,
                })
            }
            SyncAction::Resync(reason) => {
                if matches!(reason, BinanceSyncError::SnapshotDoesNotBridge { .. }) {
                    self.stale_snapshots = self.stale_snapshots.saturating_add(1);
                    if self.stale_snapshots > MAX_STALE_SNAPSHOTS {
                        return Err(BinanceStateError::Sync(reason));
                    }
                }
                self.start_sync(now_ns, outgoing)?;
                Ok(BinanceAction::Resync { reason })
            }
        }
    }

    fn sequences(&mut self) -> Result<ActorSequences, BinanceStateError> {
        let sequence = self.next_seq;
        self.next_seq = sequence
            .checked_add(1)
            .ok_or(BinanceStateError::SequenceExhausted)?;
        Ok(ActorSequences {
            stream_epoch: Some(self.epoch),
            stream_seq: Some(sequence),
        })
    }

    fn market_message(&mut self, event: MarketEvent, ts: ActorTimestamps) -> Result<ConnectorOut, BinanceStateError> {
        Ok(ConnectorOut::Market(ActorMessage {
            meta: ActorMeta { ts, seq: self.sequences()? },
            payload: event,
        }))
    }

    fn status_message(&mut self, state: FeedState, now_ns: u64) -> Result<ConnectorOut, BinanceStateError> {
        Ok(ConnectorOut::FeedStatus(ActorMessage {
            meta: ActorMeta {
                ts: ActorTimestamps {
                    received_ts_ns: Some(now_ns),
                    actor_ts_ns: Some(now_ns),
                    ..ActorTimestamps::default()
                },
                seq: self.sequences()?,
            },
            payload: FeedStatus {
                venue: Venue::Binance,
                instrument_id: self.instrument_id,
                state,
            },
        }))
    }
}

impl BinanceState {
    pub fn process_message(&mut self, event: BinanceEvent, now_ns: u64, outgoing: &mut BinanceOutput) -> Result<BinanceAction, BinanceStateError> {
        match event {
            BinanceEvent::SessionStarted => {
                self.epoch = self
                    .epoch
                    .checked_add(1)
                    .ok_or(BinanceStateError::SequenceExhausted)?;
                self.next_seq = 0;
                self.sync = None;
                self.stale_snapshots = 0;
                outgoing[0] = Some(self.status_message(FeedState::Connecting, now_ns)?);
                Ok(BinanceAction::None)
            }
            BinanceEvent::Connected => self.start_sync(now_ns, outgoing),
            BinanceEvent::Snapshot(snapshot) => {
                let ts = ActorTimestamps {
                    exchange_ts_ns: Some(snapshot.snapshot.event_ts_ns),
                    source_ts_ns: Some(snapshot.snapshot.transaction_ts_ns),
                    received_ts_ns: Some(snapshot.received_ts_ns),
                    actor_ts_ns: Some(snapshot.parsed_ts_ns),
                };
                self.snapshot_id = snapshot.snapshot.book.last_update_id;
                self.sync = Some(DepthSync::new(self.snapshot_id));
                outgoing[0] = Some(self.market_message(MarketEvent::Snapshot(snapshot.snapshot.book), ts)?);
                Ok(BinanceAction::None)
            }
            BinanceEvent::Delta(delta) => self.apply_delta(delta, now_ns, outgoing),
            BinanceEvent::Command(ConnectorIn::Resync { instrument_id }) => {
                if instrument_id != self.instrument_id {
                    return Ok(BinanceAction::IgnoredInstrument { instrument_id });
                }
                if self.sync.as_ref().is_some_and(DepthSync::is_bridged) {
                    self.start_sync(now_ns, outgoing)
                } else {
                    Ok(BinanceAction::None)
                }
            }
            BinanceEvent::Disconnected => {
                self.sync = None;
                outgoing[0] = Some(self.status_message(FeedState::Disconnected, now_ns)?);
                Ok(BinanceAction::None)
            }
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
