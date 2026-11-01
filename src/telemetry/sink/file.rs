use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Duration;

use sonic_rs::writer::BufferedWriter;

use crate::engine::EngineTelemetry;
use crate::telemetry::TelemetryRecord;

use super::TelemetrySink;

pub struct FileTelemetrySink {
    writer: BufferedWriter<BufWriter<File>>,
    latest: Option<EngineTelemetry>,
    reported_messages: u64,
}

impl FileTelemetrySink {
    pub fn new(file: File) -> Self {
        Self {
            writer: BufferedWriter::new(BufWriter::new(file)),
            latest: None,
            reported_messages: 0,
        }
    }
}

impl TelemetrySink for FileTelemetrySink {
    type Record = TelemetryRecord;
    type Error = std::io::Error;

    fn write(&mut self, record: &Self::Record) -> Result<(), Self::Error> {
        sonic_rs::to_writer(&mut self.writer, record).map_err(std::io::Error::other)?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        let TelemetryRecord::Engine(message) = record;
        self.latest = Some(message.actor.payload);
        Ok(())
    }

    fn on_interval(&mut self, elapsed: Duration) {
        let Some(metrics) = self.latest else {
            tracing::info!(msg_per_s = 0.0, "telemetry summary: waiting for engine");
            return;
        };
        let msg_per_s = metrics.messages.saturating_sub(self.reported_messages) as f64 / elapsed.as_secs_f64();
        self.reported_messages = metrics.messages;
        tracing::info!(
            msg_per_s = %format_args!("{msg_per_s:.1}"),
            messages = metrics.messages,
            signals = metrics.signals,
            resyncs = metrics.resync_requests,
            dropped = metrics.dropped_outgoing,
            processing_p90_us = ?metrics.processing.map(|latency| latency.p90_ns as f64 / 1_000.0),
            processing_p99_us = ?metrics.processing.map(|latency| latency.p99_ns as f64 / 1_000.0),
            receive_to_engine_p90_us = ?metrics.receive_to_engine.map(|latency| latency.p90_ns as f64 / 1_000.0),
            receive_to_engine_p99_us = ?metrics.receive_to_engine.map(|latency| latency.p99_ns as f64 / 1_000.0),
            receive_to_decision_p90_us = ?metrics.receive_to_decision.map(|latency| latency.p90_ns as f64 / 1_000.0),
            receive_to_decision_p99_us = ?metrics.receive_to_decision.map(|latency| latency.p99_ns as f64 / 1_000.0),
            percentile_scope = "since_start",
            "telemetry summary",
        );
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.writer.flush()
    }
}
