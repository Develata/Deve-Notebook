//! 回调函数导出层。
//!
//! 文档、杂项、同步、切换和 Source Control 回调分别在各自模块实现，
//! 这里统一做 re-export，保持 `use_core::mod` 的调用点稳定。

#[path = "callbacks_doc.rs"]
mod doc;
#[path = "callbacks_misc.rs"]
mod misc;

pub use doc::{DocCallbackSignals, DocCallbacks, create_doc_callbacks};
pub use misc::{MiscCallbacks, MiscRequestSignals, SearchScopeSignals, create_misc_callbacks};
