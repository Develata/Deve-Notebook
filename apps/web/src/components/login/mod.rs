//! plan_ref:
//!   - 09_auth#unauthorized-disconnected-ui
//!

mod api;
mod page;
mod state;
mod unavailable;

pub use page::LoginPage;
pub use state::AuthState;
pub use unavailable::AuthUnavailablePage;

pub async fn logout() -> Result<(), String> {
    api::logout().await
}
