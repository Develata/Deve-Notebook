mod api;
mod page;
mod state;
mod unavailable;

pub use page::LoginPage;
pub use state::AuthState;
pub use unavailable::AuthUnavailablePage;

#[allow(dead_code)]
pub async fn logout() -> Result<(), String> {
    api::logout().await
}
