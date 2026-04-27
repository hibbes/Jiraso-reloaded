// src-tauri/src/commands.rs
//
// Dünne Verdrahtungsschicht zwischen Tauri und den reinen Backend-Modulen.
// Enthaelt keine Business-Logik — delegiert ausschliesslich.
//
// Aus Linux-Build-Gruenden (webkit2gtk) steht dieses Modul hinter dem
// `desktop`-Feature. Backend-Tests (--no-default-features) kompilieren es
// nicht mit.

use crate::auth::{self, Rolle};
use crate::config::{self, AppConfig};
use crate::error::AppResult;
use crate::lock::{self, LockConfig as LockCfg, Session};
use std::sync::Mutex;

pub struct AppState {
    pub config_path: std::path::PathBuf,
    pub lock_path: std::path::PathBuf,
    pub config: Mutex<AppConfig>,
    pub session: Mutex<Option<Session>>,
    pub rolle: Mutex<Option<Rolle>>,
}

#[tauri::command]
pub fn login(
    state: tauri::State<AppState>,
    passwort: String,
    rechner: String,
) -> AppResult<Rolle> {
    let cfg = state.config.lock().unwrap().clone();
    let rolle = auth::resolve_role(&passwort, &cfg)?
        .ok_or_else(|| crate::error::AppError::Config("Falsches Passwort".into()))?;

    let rolle_str = match rolle {
        Rolle::Fachlehrer => "Fachlehrer",
        Rolle::Klassenlehrer => "Klassenlehrer",
        Rolle::Administrator => "Administrator",
    };

    let lock_cfg = LockCfg {
        stale_hours: cfg.lock.stale_hours,
        max_parallel: cfg.lock.max_parallel,
    };
    let session = lock::acquire(&state.lock_path, &rechner, rolle_str, &lock_cfg)?;

    *state.session.lock().unwrap() = Some(session);
    *state.rolle.lock().unwrap() = Some(rolle);
    Ok(rolle)
}

#[tauri::command]
pub fn logout(state: tauri::State<AppState>) -> AppResult<()> {
    if let Some(s) = state.session.lock().unwrap().take() {
        lock::release(&state.lock_path, &s.id)?;
    }
    *state.rolle.lock().unwrap() = None;
    Ok(())
}

#[tauri::command]
pub fn current_role(state: tauri::State<AppState>) -> Option<Rolle> {
    *state.rolle.lock().unwrap()
}

#[tauri::command]
pub fn schulname(state: tauri::State<AppState>) -> String {
    state.config.lock().unwrap().schule.clone()
}

#[tauri::command]
pub fn aktuelles_schuljahr(state: tauri::State<AppState>) -> String {
    state.config.lock().unwrap().aktuelles_schuljahr.clone()
}

#[tauri::command]
pub fn break_lock_admin(
    state: tauri::State<AppState>,
    admin_passwort: String,
) -> AppResult<()> {
    let cfg = state.config.lock().unwrap().clone();
    if !auth::verify_password(&admin_passwort, &cfg.passwoerter.administrator)? {
        return Err(crate::error::AppError::Config("Admin-Passwort falsch".into()));
    }
    lock::break_lock(&state.lock_path)
}

#[tauri::command]
pub fn needs_setup(state: tauri::State<AppState>) -> bool {
    config::passwoerter_leer(&state.config.lock().unwrap())
}

#[tauri::command]
pub fn setup_passwoerter(
    state: tauri::State<AppState>,
    fachlehrer: String,
    klassenlehrer: String,
    administrator: String,
) -> AppResult<()> {
    if fachlehrer.len() < 8 || klassenlehrer.len() < 8 || administrator.len() < 8 {
        return Err(crate::error::AppError::Config(
            "Alle drei Passwörter müssen mindestens 8 Zeichen haben.".into(),
        ));
    }

    let mut cfg = state.config.lock().unwrap();
    cfg.passwoerter.fachlehrer = auth::hash_password(&fachlehrer)?;
    cfg.passwoerter.klassenlehrer = auth::hash_password(&klassenlehrer)?;
    cfg.passwoerter.administrator = auth::hash_password(&administrator)?;
    config::save(&cfg, &state.config_path)?;
    Ok(())
}

use crate::error::AppError;
use crate::import::{self, ColumnMapping, DetectResult, ParsedSheet};
use crate::stammdaten::{self, ImportSummary, Schuljahr};

fn require_admin(state: &tauri::State<AppState>) -> AppResult<()> {
    match *state.rolle.lock().unwrap() {
        Some(Rolle::Administrator) => Ok(()),
        _ => Err(AppError::Locked("Nur Administrator:innen dürfen Stammdaten ändern".into())),
    }
}

fn open_db(state: &tauri::State<AppState>) -> AppResult<rusqlite::Connection> {
    // Pfad wird aus dem AppState-Lockpath abgeleitet (data-Ordner neben lock)
    let data_dir = state.lock_path.parent()
        .ok_or_else(|| AppError::Config("Lock-Pfad ohne Parent".into()))?;
    crate::db::open(&data_dir.join("jiraso.db"))
}

#[tauri::command]
pub fn list_schuljahre(state: tauri::State<AppState>) -> AppResult<Vec<Schuljahr>> {
    let conn = open_db(&state)?;
    stammdaten::list_schuljahre(&conn)
}

#[tauri::command]
pub fn schuljahr_anlegen(
    bezeichnung: String,
    aktivieren: bool,
    state: tauri::State<AppState>,
) -> AppResult<i64> {
    require_admin(&state)?;
    let mut conn = open_db(&state)?;
    let id = stammdaten::anlegen(&conn, &bezeichnung)?;
    if aktivieren {
        stammdaten::aktivieren(&mut conn, id)?;
    }
    Ok(id)
}

#[tauri::command]
pub fn schuljahr_aktivieren(
    id: i64,
    state: tauri::State<AppState>,
) -> AppResult<()> {
    require_admin(&state)?;
    let mut conn = open_db(&state)?;
    stammdaten::aktivieren(&mut conn, id)
}

#[derive(serde::Serialize)]
pub struct ImportPreview {
    pub sheet: ParsedSheet,
    pub detection: DetectResult,
}

#[tauri::command]
pub fn import_preview(
    bytes: Vec<u8>,
    state: tauri::State<AppState>,
) -> AppResult<ImportPreview> {
    require_admin(&state)?;
    let sheet = import::parse_sheet(&bytes)?;
    let detection = import::detect_columns(&sheet.headers);
    Ok(ImportPreview { sheet, detection })
}

#[tauri::command]
pub fn import_apply(
    schuljahr_id: i64,
    bytes: Vec<u8>,
    mapping: ColumnMapping,
    state: tauri::State<AppState>,
) -> AppResult<ImportSummary> {
    require_admin(&state)?;
    let sheet = import::parse_sheet(&bytes)?;
    let records = import::build_inputs(&sheet, &mapping);
    let mut conn = open_db(&state)?;
    stammdaten::upsert_schueler(&mut conn, schuljahr_id, &records)
}
