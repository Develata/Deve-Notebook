// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Bottom Bar Module (底部栏翻译)

mod labels;
mod loading;
mod playback;
mod status;

pub use labels::*;
pub use loading::*;
pub use playback::*;
pub use status::*;

#[cfg(test)]
mod tests;
