//! Source Control Handlers
//!
//! Refactored into submodules for maintainability.

pub mod changes;
pub mod commits;
mod commits_query;
mod commits_write;
pub mod conflict;
pub mod diff;
pub mod discard;
mod errors;
pub mod http;
pub mod http_commits;
pub mod http_mutations;
mod local_discard;
mod present;
mod repo_scope;
mod service;
pub mod staging;

pub use changes::*;
pub use commits::*;
pub use conflict::*;
pub use diff::*;
pub use discard::*;
pub use staging::*;
