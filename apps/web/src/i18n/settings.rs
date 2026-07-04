// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Settings Module (设置翻译)

mod ai;
mod backend;
mod core;
mod local_prefs;
mod native_backend;
mod sync;

pub use ai::*;
pub use backend::*;
pub use core::*;
pub use local_prefs::*;
pub use native_backend::*;
pub use sync::*;

#[cfg(test)]
mod tests;
