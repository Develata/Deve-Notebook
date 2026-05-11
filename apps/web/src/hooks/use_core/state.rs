// apps/web/src/hooks/use_core/state.rs
//! # 状态信号声明
//!
//! 定义 `use_core` 所需的所有响应式信号。

mod core_signals;
mod state_types;

pub use super::state_init::init_signals;
pub use core_signals::CoreSignals;
pub use state_types::PluginResponse;
