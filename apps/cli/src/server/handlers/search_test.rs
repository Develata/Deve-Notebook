#[cfg(feature = "search")]
#[path = "search_test/support_test.rs"]
mod support;

#[cfg(feature = "search")]
#[path = "search_test/feature_enabled_test.rs"]
mod feature_enabled;

#[cfg(not(feature = "search"))]
#[path = "search_test/feature_disabled_test.rs"]
mod feature_disabled;
