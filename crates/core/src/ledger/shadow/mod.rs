//! # 影子库模块 (Shadow Repository Module)
//!
//! **架构作用**:
//! 聚合影子库的访问层 (View) 和管理层 (Management)。
//!
//! **类型**: Core MUST (核心必选)

pub mod access;
pub mod management;

pub use access::ShadowRepo;
pub use management::{ensure_shadow_db, list_shadows_on_disk};
