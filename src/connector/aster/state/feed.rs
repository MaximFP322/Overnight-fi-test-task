use crate::connector::{ConnectorIn, ConnectorOut};
use crate::instrument::InstrumentId;
use crate::market::{FeedState, FeedStatus, MarketEvent, Venue};
use crate::message::{ActorMessage, ActorMeta, ActorSequences, ActorTimestamps};

use super::super::AsterDepthDelta;
use super::sync::{DepthSync, SyncAction};
use super::{AsterAction, AsterEvent, AsterOutput, AsterStateError, AsterSyncError};

const MAX_STALE_SNAPSHOTS: u32 = 3;

pub struct AsterState {
    instrument_id: InstrumentId,
    epoch: u64,
    next_seq: u64,
    sync: Option<DepthSync>,
    snapshot_id: u64,
    stale_snapshots: u32,
}

impl AsterState {
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

    fn start_sync(&mut self, now_ns: u64, outgoing: &mut AsterOutput) -> Result<AsterAction, AsterStateError> {
        self.sync = None;
        outgoing[0] = Some(self.status_message(FeedState::Syncing, now_ns)?);
        Ok(AsterAction::FetchSnapshot)
    }

    fn apply_delta(&mut self, delta: ActorMessage<AsterDepthDelta>, now_ns: u64, outgoing: &mut AsterOutput) -> Result<AsterAction, AsterStateError> {
        let sync = self
            .sync
            .as_mut()
            .ok_or(AsterStateError::Sync(AsterSyncError::MissingSnapshot))?;
        let action = sync.apply(&delta.payload.book);
        let last_update_id = delta.payload.book.final_update_id;
        match action {
            SyncAction::Ignore => Ok(AsterAction::None),
            SyncAction::Apply => {
                outgoing[0] = Some(self.market_message(MarketEvent::Delta(delta.payload.book), delta.meta.ts)?);
                Ok(AsterAction::None)
            }
            SyncAction::Bridge => {
                self.stale_snapshots = 0;
                outgoing[0] = Some(self.market_message(MarketEvent::Delta(delta.payload.book), delta.meta.ts)?);
                outgoing[1] = Some(self.status_message(FeedState::Live, now_ns)?);
                Ok(AsterAction::Synchronized {
                    snapshot_id: self.snapshot_id,
                    last_update_id,
                })
            }
            SyncAction::Resync(reason) => {
                if matches!(reason, AsterSyncError::SnapshotDoesNotBridge { .. }) {
                    self.stale_snapshots = self.stale_snapshots.saturating_add(1);
                    if self.stale_snapshots > MAX_STALE_SNAPSHOTS {
                        return Err(AsterStateError::Sync(reason));
                    }
                }
                self.start_sync(now_ns, outgoing)?;
                Ok(AsterAction::Resync { reason })
            }
        }
    }

    fn sequences(&mut self) -> Result<ActorSequences, AsterStateError> {
        let sequence = self.next_seq;
        self.next_seq = sequence
            .checked_add(1)
            .ok_or(AsterStateError::SequenceExhausted)?;
        Ok(ActorSequences {
            stream_epoch: Some(self.epoch),
            stream_seq: Some(sequence),
        })
    }

    fn market_message(&mut self, event: MarketEvent, ts: ActorTimestamps) -> Result<ConnectorOut, AsterStateError> {
        Ok(ConnectorOut::Market(ActorMessage {
            meta: ActorMeta { ts, seq: self.sequences()? },
            payload: event,
        }))
    }

    fn status_message(&mut self, state: FeedState, now_ns: u64) -> Result<ConnectorOut, AsterStateError> {
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
                venue: Venue::Aster,
                instrument_id: self.instrument_id,
                state,
            },
        }))
    }
}

impl AsterState {
    pub fn process_message(&mut self, event: AsterEvent, now_ns: u64, outgoing: &mut AsterOutput) -> Result<AsterAction, AsterStateError> {
        match event {
            AsterEvent::SessionStarted => {
                self.epoch = self
                    .epoch
                    .checked_add(1)
                    .ok_or(AsterStateError::SequenceExhausted)?;
                self.next_seq = 0;
                self.sync = None;
                self.stale_snapshots = 0;
                outgoing[0] = Some(self.status_message(FeedState::Connecting, now_ns)?);
                Ok(AsterAction::None)
            }
            AsterEvent::Connected => self.start_sync(now_ns, outgoing),
            AsterEvent::Snapshot(snapshot) => {
                let ts = ActorTimestamps {
                    exchange_ts_ns: Some(snapshot.snapshot.event_ts_ns),
                    source_ts_ns: Some(snapshot.snapshot.transaction_ts_ns),
                    received_ts_ns: Some(snapshot.received_ts_ns),
                    actor_ts_ns: Some(snapshot.parsed_ts_ns),
                };
                self.snapshot_id = snapshot.snapshot.book.last_update_id;
                self.sync = Some(DepthSync::new(self.snapshot_id));
                outgoing[0] = Some(self.market_message(MarketEvent::Snapshot(snapshot.snapshot.book), ts)?);
                Ok(AsterAction::None)
            }
            AsterEvent::Delta(delta) => self.apply_delta(delta, now_ns, outgoing),
            AsterEvent::Command(ConnectorIn::Resync { instrument_id }) => {
                if instrument_id != self.instrument_id {
                    return Ok(AsterAction::IgnoredInstrument { instrument_id });
                }
                if self.sync.as_ref().is_some_and(DepthSync::is_bridged) {
                    self.start_sync(now_ns, outgoing)
                } else {
                    Ok(AsterAction::None)
                }
            }
            AsterEvent::Disconnected => {
                self.sync = None;
                outgoing[0] = Some(self.status_message(FeedState::Disconnected, now_ns)?);
                Ok(AsterAction::None)
            }
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
