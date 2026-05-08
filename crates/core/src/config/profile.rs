//! plan_ref:
//!   - 13_settings#configuration-settings
//!
//! Runtime profile preset application.

use super::{AppProfile, Config};

pub(super) fn apply_profile_presets(settings: &config::Config, config: &mut Config) {
    if !has_explicit_key(settings, "snapshot_depth") {
        config.snapshot_depth = profile_snapshot_depth(config.profile);
    }
    if !has_explicit_key(settings, "mem_cache_mb") && std::env::var_os("MEM_CACHE_MB").is_none() {
        config.mem_cache_mb = profile_mem_cache_mb(config.profile);
    }
}

fn has_explicit_key(settings: &config::Config, key: &str) -> bool {
    settings.get::<config::Value>(key).is_ok()
}

fn profile_snapshot_depth(profile: AppProfile) -> usize {
    match profile {
        AppProfile::Standard => 100,
        AppProfile::LowSpec => 10,
    }
}

fn profile_mem_cache_mb(profile: AppProfile) -> usize {
    match profile {
        AppProfile::Standard => 128,
        AppProfile::LowSpec => 32,
    }
}
