// Jiraso-reloaded library crate root.
//
// Backend modules (db, config, auth, lock, backup, error) are plain Rust
// and can be unit-tested without the tauri runtime via:
//
//     cargo test --no-default-features --lib
//
// The `desktop` feature (default-on) adds the tauri runtime and the
// `commands` module. On Linux this pulls webkit2gtk — CI provides it,
// local dev without system libs should disable the feature.

pub mod error;

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
