use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EngineTelemetry {
    pub messages: u64,
    pub signals: u64,
    pub resync_requests: u64,
    pub dropped_outgoing: u64,
    pub processing: Option<LatencySummary>,
    pub parse: Option<LatencySummary>,
    pub handoff: Option<LatencySummary>,
    pub receive_to_engine: Option<LatencySummary>,
    pub receive_to_decision: Option<LatencySummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LatencySummary {
    pub count: u64,
    pub min_ns: u64,
    pub p50_ns: u64,
    pub p90_ns: u64,
    pub p99_ns: u64,
    pub max_ns: u64,
}
