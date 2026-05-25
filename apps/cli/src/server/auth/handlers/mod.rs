//! plan_ref:
//!   - 08_auth#auth-http-endpoints
//!   - 08_auth#jwt-cookie-contract

mod login;
mod native_session;
mod session;

pub use login::login;
pub use native_session::{NativeSessionBridge, native_session};
pub use session::{logout, me, status};
