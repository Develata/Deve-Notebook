fn main() {
    #[cfg(feature = "native-packaging")]
    tauri_build::build();
}
