use leptos::prelude::*;

#[derive(Clone)]
pub struct DiffMetricsState {
    pub last_compute_ms: ReadSignal<u32>,
    pub set_last_compute_ms: WriteSignal<u32>,
    pub cache_hit: ReadSignal<bool>,
    pub set_cache_hit: WriteSignal<bool>,
    pub cache_hits: ReadSignal<u32>,
    pub set_cache_hits: WriteSignal<u32>,
    pub cache_total: ReadSignal<u32>,
    pub set_cache_total: WriteSignal<u32>,
    pub cache_hit_ratio: ReadSignal<u32>,
    pub set_cache_hit_ratio: WriteSignal<u32>,
    pub algorithm: ReadSignal<String>,
    pub set_algorithm: WriteSignal<String>,
}

pub fn create_metrics_state() -> DiffMetricsState {
    let (last_compute_ms, set_last_compute_ms) = signal(0u32);
    let (cache_hit, set_cache_hit) = signal(false);
    let (cache_hits, set_cache_hits) = signal(0u32);
    let (cache_total, set_cache_total) = signal(0u32);
    let (cache_hit_ratio, set_cache_hit_ratio) = signal(0u32);
    let (algorithm, set_algorithm) = signal("Myers".to_string());
    DiffMetricsState {
        last_compute_ms,
        set_last_compute_ms,
        cache_hit,
        set_cache_hit,
        cache_hits,
        set_cache_hits,
        cache_total,
        set_cache_total,
        cache_hit_ratio,
        set_cache_hit_ratio,
        algorithm,
        set_algorithm,
    }
}

pub fn record_cache_sample(metrics: &DiffMetricsState, hit: bool) {
    metrics.set_cache_hit.set(hit);
    let hits = metrics.cache_hits.get_untracked();
    let total = metrics.cache_total.get_untracked();
    let next_hits = hits + u32::from(hit);
    let next_total = total.saturating_add(1);
    let ratio = if next_total == 0 {
        0
    } else {
        (next_hits.saturating_mul(100)) / next_total
    };
    metrics.set_cache_hits.set(next_hits);
    metrics.set_cache_total.set(next_total);
    metrics.set_cache_hit_ratio.set(ratio);
}

pub fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}

pub fn elapsed_ms(start: u64, end: u64) -> u32 {
    end.saturating_sub(start) as u32
}
