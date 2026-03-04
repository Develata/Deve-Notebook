//! Source Control Handlers
//!
//! Refactored into submodules for maintainability.

pub mod changes;
pub mod commits;
pub mod diff;
pub mod discard;
pub mod http;
pub mod staging;

pub use changes::*;
pub use commits::*;
pub use diff::*;
pub use discard::*;
pub use staging::*;
