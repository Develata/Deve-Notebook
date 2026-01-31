#[derive(Clone, Copy, Debug)]
pub struct SnapshotPolicy {
    pub min_interval: u64,
    pub max_interval: u64,
}

impl SnapshotPolicy {
    pub fn default() -> Self {
        Self {
            min_interval: 16,
            max_interval: 256,
        }
    }

    pub fn should_snapshot(&self, doc_len: usize, ops_delta: u64, last_open_ms: u64) -> bool {
        let len_factor = if doc_len > 200_000 {
            16
        } else if doc_len > 50_000 {
            32
        } else {
            64
        };

        let perf_factor = if last_open_ms > 500 {
            16
        } else if last_open_ms > 200 {
            32
        } else {
            64
        };

        let interval = len_factor
            .min(perf_factor)
            .clamp(self.min_interval, self.max_interval);
        ops_delta >= interval
    }
}
