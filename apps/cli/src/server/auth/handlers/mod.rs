//! plan_ref:
//!   - 09_auth#auth-http-endpoints
//!   - 09_auth#jwt-cookie-contract

mod login;
mod native_session;
mod session;

pub use login::login;
pub use native_session::{NativeSessionBridge, native_session};
pub use session::{logout, me, status};
