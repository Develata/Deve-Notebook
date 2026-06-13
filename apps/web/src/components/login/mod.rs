//! plan_ref:
//!   - 08_auth#unauthorized-disconnected-ui
//!

mod page;
mod state;
mod unavailable;

pub use page::LoginPage;
pub use state::AuthState;
pub use unavailable::AuthUnavailablePage;

pub async fn logout() -> Result<(), String> {
    crate::api::logout().await
}
