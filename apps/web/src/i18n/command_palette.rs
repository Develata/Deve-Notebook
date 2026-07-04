// apps\web\src\i18n
//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! # I18n Command Palette Module (命令面板翻译)

#![allow(dead_code)] // 翻译字符串按需使用

mod ai;
mod core;
mod git;
mod keyboard;
mod metadata;
mod peer;
mod remote_projection;
mod source_control;

pub use ai::*;
pub use core::*;
pub use git::*;
pub use keyboard::*;
pub use metadata::*;
pub use peer::*;
pub use remote_projection::*;
pub use source_control::*;

#[cfg(test)]
mod tests;
