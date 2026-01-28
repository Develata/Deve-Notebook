//! Source Control Handlers
//!
//! Refactored into submodules for maintainability.

pub mod changes;
pub mod commits;
pub mod diff;
pub mod http;
pub mod staging;

pub use changes::*;
pub use commits::*;
pub use diff::*;
pub use staging::*;
