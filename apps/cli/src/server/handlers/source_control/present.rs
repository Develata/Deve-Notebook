//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

#[path = "present_paths.rs"]
mod paths;
#[path = "present_resolve.rs"]
mod resolve;

pub use paths::{collapse_rename_candidates, expand_related_paths};
pub(crate) use resolve::resolve_target_path_strict;

#[cfg(test)]
#[path = "present_related_test.rs"]
mod related_tests;
#[cfg(test)]
#[path = "present_resolve_extra_test.rs"]
mod resolve_extra_tests;
#[cfg(test)]
#[path = "present_resolve_test.rs"]
mod resolve_tests;
#[cfg(test)]
#[path = "present_test_support.rs"]
mod test_support;
