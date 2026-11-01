use std::time::{SystemTime, UNIX_EPOCH};

use super::Clock;

#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ns(&self) -> u64 {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
    }
}
