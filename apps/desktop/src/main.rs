#[cfg(feature = "native-packaging")]
fn main() -> tauri::Result<()> {
    deve_desktop::run_desktop_tauri_app()
}

#[cfg(not(feature = "native-packaging"))]
fn main() {
    eprintln!("deve_desktop native runtime is disabled; rebuild with --features native-packaging");
}
