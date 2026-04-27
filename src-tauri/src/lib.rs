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
pub mod db;
pub mod config;
pub mod auth;
pub mod lock;
pub mod backup;
pub mod stammdaten;
pub mod import;

#[cfg(feature = "desktop")]
pub mod commands;

#[cfg(feature = "desktop")]
use crate::error::AppResult;
#[cfg(feature = "desktop")]
use std::sync::Mutex;

#[cfg(feature = "desktop")]
fn app_root() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

#[cfg(feature = "desktop")]
fn run_setup() -> AppResult<commands::AppState> {
    let root = app_root();
    let config_path = root.join("config.toml");
    let data_dir = root.join("data");
    let backups_dir = data_dir.join("backups");
    std::fs::create_dir_all(&data_dir)?;

    let cfg = config::load_or_create(&config_path)?;

    let db_path = data_dir.join("jiraso.db");
    let _conn = db::open(&db_path)?;
    drop(_conn);

    backup::daily_backup(&db_path, &backups_dir)?;

    Ok(commands::AppState {
        config_path,
        lock_path: data_dir.join("jiraso.db.lock"),
        config: Mutex::new(cfg),
        session: Mutex::new(None),
        rolle: Mutex::new(None),
    })
}

#[cfg(feature = "desktop")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = run_setup().expect("Setup fehlgeschlagen");

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::logout,
            commands::current_role,
            commands::schulname,
            commands::aktuelles_schuljahr,
            commands::break_lock_admin,
            commands::needs_setup,
            commands::setup_passwoerter,
            commands::list_schuljahre,
            commands::schuljahr_anlegen,
            commands::schuljahr_aktivieren,
            commands::import_preview,
            commands::import_apply,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri-App konnte nicht starten");
}
