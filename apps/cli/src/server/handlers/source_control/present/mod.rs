//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

mod paths;
mod resolve;

pub use paths::collapse_rename_candidates;
#[cfg(test)]
pub use paths::expand_related_paths;
pub(crate) use resolve::resolve_target_path_strict;

#[cfg(test)]
mod related_tests;
#[cfg(test)]
mod resolve_extra_tests;
#[cfg(test)]
mod resolve_tests;
#[cfg(test)]
mod test_support;
