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
pub mod katalog;
pub mod bewertung;
pub mod bemerkung;
pub mod uebersicht;
pub mod goodies;
pub mod bug_report;

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
        .plugin(tauri_plugin_dialog::init())
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
            commands::katalog_faecher,
            commands::katalog_kategorien,
            commands::katalog_formulierungen,
            commands::katalog_fach_anlegen,
            commands::katalog_seed_default_faecher,
            commands::katalog_seed_default_floskeln,
            commands::katalog_kategorie_anlegen,
            commands::katalog_formulierung_anlegen,
            commands::katalog_fach_aktiv,
            commands::katalog_kategorie_aktiv,
            commands::katalog_formulierung_aktiv,
            commands::katalog_fach_reihenfolge,
            commands::katalog_kategorie_reihenfolge,
            commands::katalog_formulierung_reihenfolge,
            commands::bewertung_matrix,
            commands::bewertung_set,
            commands::bewertung_wuerfeln,
            commands::bemerkung_get,
            commands::bemerkung_set,
            commands::klassenraum_klassen,
            commands::klassenraum_schueler,
            commands::uebersicht_schueler,
            commands::uebersicht_klasse,
            commands::goodies_zitat,
            commands::bug_report_submit,
        ])
        .run(tauri::generate_context!())
        .expect("Tauri-App konnte nicht starten");
}
