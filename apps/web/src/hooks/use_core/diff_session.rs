//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 10_rendering#document-authority-bridge
//!
//! Compatibility re-export for the source-control client diff session type.

pub use crate::runtime::source_control_client::diff_session::{
    DiffSessionWire, MergeConflictSession,
};
