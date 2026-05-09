#[cfg(feature = "search")]
mod support;

#[cfg(feature = "search")]
mod feature_enabled;

#[cfg(not(feature = "search"))]
mod feature_disabled;
