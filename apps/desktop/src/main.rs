//! plan_ref:
//!   - 17_tech_stack#native-packaging-dependency-gate
//!   - 11_ui_design/02_desktop#desktop-packaging-scaffold

#[cfg(feature = "native-packaging")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("DEVE_DESKTOP_STARTUP_SMOKE").is_some() {
        let smoke = deve_desktop::desktop_tauri_startup_smoke();
        if !smoke.passed() {
            return Err(std::io::Error::other("desktop startup smoke failed").into());
        }
        println!("{}", deve_desktop::DESKTOP_TAURI_STARTUP_SMOKE_OK);
        return Ok(());
    }
    if std::env::var_os("DEVE_DESKTOP_NATIVE_SESSION_SMOKE").is_some() {
        let smoke = deve_desktop::desktop_tauri_native_session_smoke(0)?;
        if !smoke.passed() {
            return Err(std::io::Error::other("desktop native session smoke failed").into());
        }
        println!("{}", deve_desktop::DESKTOP_TAURI_NATIVE_SESSION_SMOKE_OK);
        return Ok(());
    }

    let launch_options =
        deve_desktop::DesktopTauriLaunchOptions::from_args(std::env::args().skip(1))
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;

    deve_desktop::run_desktop_tauri_app_with_launch_options(launch_options)?;
    Ok(())
}

#[cfg(not(feature = "native-packaging"))]
fn main() {
    eprintln!("deve_desktop native runtime is disabled; rebuild with --features native-packaging");
}
