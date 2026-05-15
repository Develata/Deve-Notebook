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

    deve_desktop::run_desktop_tauri_app()?;
    Ok(())
}

#[cfg(not(feature = "native-packaging"))]
fn main() {
    eprintln!("deve_desktop native runtime is disabled; rebuild with --features native-packaging");
}
