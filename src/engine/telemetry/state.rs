use crate::engine::{EngineOut, EngineOutput};
use crate::message::{ActorMessage, ActorMeta, EngineMessage};

use super::EngineTelemetry;
use super::histogram::Histogram;

const TELEMETRY_INTERVAL_NS: u64 = 1_000_000_000;

struct TelemetryMeta {
    meta: ActorMeta,
    engine_ts_ns: u64,
    engine_seq: u64,
}

pub struct EngineMetrics {
    messages: u64,
    signals: u64,
    resync_requests: u64,
    dropped_outgoing: u64,
    processing: Histogram,
    parse: Histogram,
    handoff: Histogram,
    receive_to_engine: Histogram,
    receive_to_decision: Histogram,
    last_snapshot_ts_ns: Option<u64>,
    last_event: Option<TelemetryMeta>,
}

impl Default for EngineMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineMetrics {
    pub fn new() -> Self {
        Self {
            messages: 0,
            signals: 0,
            resync_requests: 0,
            dropped_outgoing: 0,
            processing: Histogram::new(),
            parse: Histogram::new(),
            handoff: Histogram::new(),
            receive_to_engine: Histogram::new(),
            receive_to_decision: Histogram::new(),
            last_snapshot_ts_ns: None,
            last_event: None,
        }
    }

    pub(crate) fn record_message(
        &mut self,
        meta: ActorMeta,
        engine_ts_ns: u64,
        engine_seq: u64,
        processing_ns: u64,
        outgoing: &EngineOutput,
    ) -> Option<EngineOut> {
        let timestamps = meta.ts;
        self.messages = self.messages.saturating_add(1);
        for message in outgoing.iter().flatten() {
            match message {
                EngineOut::Signal(_) => self.signals = self.signals.saturating_add(1),
                EngineOut::Resync(_) => self.resync_requests = self.resync_requests.saturating_add(1),
                EngineOut::Telemetry(_) => {}
            }
        }
        if let Some(duration) = timestamps
            .actor_ts_ns
            .zip(timestamps.received_ts_ns)
            .and_then(|(actor, received)| actor.checked_sub(received))
        {
            self.parse.record(duration);
        }
        if let Some(duration) = timestamps
            .actor_ts_ns
            .and_then(|actor| engine_ts_ns.checked_sub(actor))
        {
            self.handoff.record(duration);
        }
        if let Some(duration) = timestamps
            .received_ts_ns
            .and_then(|received| engine_ts_ns.checked_sub(received))
        {
            self.receive_to_engine.record(duration);
            if let Some(total_ns) = duration.checked_add(processing_ns) {
                self.receive_to_decision.record(total_ns);
            }
        }
        self.processing.record(processing_ns);
        self.last_event = Some(TelemetryMeta {
            meta,
            engine_ts_ns,
            engine_seq,
        });
        let last_snapshot_ts_ns = *self.last_snapshot_ts_ns.get_or_insert(engine_ts_ns);
        if engine_ts_ns.saturating_sub(last_snapshot_ts_ns) < TELEMETRY_INTERVAL_NS {
            return None;
        }
        self.last_snapshot_ts_ns = Some(engine_ts_ns);
        self.snapshot()
    }

    pub(crate) fn finish(&mut self) -> Option<EngineOut> {
        let summary = self.snapshot();
        self.last_event = None;
        summary
    }

    fn snapshot(&self) -> Option<EngineOut> {
        let event = self.last_event.as_ref()?;
        let payload = EngineTelemetry {
            messages: self.messages,
            signals: self.signals,
            resync_requests: self.resync_requests,
            dropped_outgoing: self.dropped_outgoing,
            processing: self.processing.summary(),
            parse: self.parse.summary(),
            handoff: self.handoff.summary(),
            receive_to_engine: self.receive_to_engine.summary(),
            receive_to_decision: self.receive_to_decision.summary(),
        };
        Some(EngineOut::Telemetry(EngineMessage {
            actor: ActorMessage { meta: event.meta, payload },
            engine_ts_ns: event.engine_ts_ns,
            engine_seq: event.engine_seq,
        }))
    }

    pub(crate) fn record_delivery_failure(&mut self) {
        self.dropped_outgoing = self.dropped_outgoing.saturating_add(1);
    }
}
