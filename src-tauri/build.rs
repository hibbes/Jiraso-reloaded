fn main() {
    // `tauri_build::build()` is only needed when the desktop runtime is being
    // built. When running pure backend unit tests via
    // `cargo test --no-default-features --lib`, tauri is not compiled and
    // its build prep would fail looking for its frontend config.
    #[cfg(feature = "desktop")]
    tauri_build::build();
}
