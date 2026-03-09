//! Source Control Handlers
//!
//! Refactored into submodules for maintainability.

pub mod changes;
pub mod commits;
pub mod conflict;
pub mod diff;
pub mod discard;
pub mod http;
mod repo_scope;
mod service;
pub mod staging;

pub use changes::*;
pub use commits::*;
pub use conflict::*;
pub use diff::*;
pub use discard::*;
pub use staging::*;
