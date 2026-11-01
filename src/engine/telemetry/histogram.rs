use super::LatencySummary;

pub(super) struct Histogram {
    inner: hdrhistogram::Histogram<u64>,
    min_ns: u64,
    max_ns: u64,
}

impl Histogram {
    pub(super) fn new() -> Self {
        Self {
            inner: hdrhistogram::Histogram::new_with_bounds(1, u64::MAX, 1).expect("fixed full-u64 bounds and one significant digit are valid"),
            min_ns: u64::MAX,
            max_ns: 0,
        }
    }

    pub(super) fn record(&mut self, value: u64) {
        self.inner
            .record(value)
            .expect("fixed full-u64 histogram accepts every u64 value");
        self.min_ns = self.min_ns.min(value);
        self.max_ns = self.max_ns.max(value);
    }

    pub(super) fn summary(&self) -> Option<LatencySummary> {
        if self.inner.is_empty() {
            return None;
        }
        let quantile = |q| {
            self.inner
                .value_at_quantile(q)
                .clamp(self.min_ns, self.max_ns)
        };
        Some(LatencySummary {
            count: self.inner.len(),
            min_ns: self.min_ns,
            p50_ns: quantile(0.5),
            p90_ns: quantile(0.9),
            p99_ns: quantile(0.99),
            max_ns: self.max_ns,
        })
    }
}
