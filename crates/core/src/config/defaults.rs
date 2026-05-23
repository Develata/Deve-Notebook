//! plan_ref:
//!   - 13_settings#configuration-settings
//!
//! Runtime config defaults shared by serde field fallback and `Config::default`.

use super::AppProfile;

pub(super) fn profile() -> AppProfile {
    AppProfile::Standard
}

pub(super) fn ledger() -> String {
    "ledger".to_string()
}

pub(super) fn snapshot_depth() -> usize {
    100
}

pub(super) fn mem_cache_mb() -> usize {
    128
}

pub(super) fn concurrency() -> usize {
    4
}

pub(super) fn ui_locale() -> String {
    "auto".to_string()
}

pub(super) fn ui_theme() -> String {
    "auto".to_string()
}

pub(super) fn true_value() -> bool {
    true
}

pub(super) fn outline_width() -> usize {
    260
}

pub(super) fn sidebar_width() -> usize {
    250
}

pub(super) fn right_panel_width() -> usize {
    350
}

pub(super) fn outer_gutter() -> usize {
    16
}

pub(super) fn recent_commands_count() -> usize {
    3
}

pub(super) fn recent_docs_count() -> usize {
    10
}

pub(super) fn ai_mode() -> String {
    "native".to_string()
}

pub(super) fn ai_native_enabled() -> bool {
    true
}

pub(super) fn agent_bridge_timeout_ms() -> u64 {
    30_000
}
