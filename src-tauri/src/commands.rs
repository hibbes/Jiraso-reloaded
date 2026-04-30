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

/// Login als Fachlehrkraft OHNE Passwort. Der Computer ist physisch
/// passwortgeschützt; ein zusätzliches App-Passwort für den 90%-Anwendungsfall
/// (Kreuze setzen) ist Reibung ohne Sicherheitsgewinn. Klassenlehrkraft +
/// Administrator brauchen weiterhin Passwort.
#[tauri::command]
pub fn login_fachlehrer(
    state: tauri::State<AppState>,
    rechner: String,
) -> AppResult<Rolle> {
    let cfg = state.config.lock().unwrap().clone();
    let lock_cfg = LockCfg {
        stale_hours: cfg.lock.stale_hours,
        max_parallel: cfg.lock.max_parallel,
    };
    let session = lock::acquire(&state.lock_path, &rechner, "Fachlehrer", &lock_cfg)?;
    *state.session.lock().unwrap() = Some(session);
    *state.rolle.lock().unwrap() = Some(Rolle::Fachlehrer);
    Ok(Rolle::Fachlehrer)
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
use crate::stammdaten::{self, ImportSummary, Klasse, SchuelerMini, Schuljahr};

fn require_admin(state: &tauri::State<AppState>) -> AppResult<()> {
    match *state.rolle.lock().unwrap() {
        Some(Rolle::Administrator) => Ok(()),
        _ => Err(AppError::Locked("Nur Administrator:innen dürfen Stammdaten ändern".into())),
    }
}

fn require_klassenlehrer_oder_admin(state: &tauri::State<AppState>) -> AppResult<()> {
    match *state.rolle.lock().unwrap() {
        Some(Rolle::Klassenlehrer) | Some(Rolle::Administrator) => Ok(()),
        _ => Err(AppError::Locked("Nur Klassenlehrer:innen oder Admin dürfen Bemerkungen schreiben".into())),
    }
}

fn require_lehrer(state: &tauri::State<AppState>) -> AppResult<()> {
    match *state.rolle.lock().unwrap() {
        Some(_) => Ok(()),
        None => Err(AppError::Locked("Nicht eingeloggt".into())),
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

// --- Katalog-Commands ---

use crate::katalog::{self, Fach, FloskelnSeedSummary, Formulierung, Kategorie, SeedSummary};

#[tauri::command]
pub fn katalog_faecher(schuljahr_id: i64, state: tauri::State<AppState>) -> AppResult<Vec<Fach>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    katalog::list_faecher(&conn, schuljahr_id)
}

#[tauri::command]
pub fn katalog_kategorien(schuljahr_id: i64, state: tauri::State<AppState>) -> AppResult<Vec<Kategorie>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    katalog::list_kategorien(&conn, schuljahr_id)
}

#[tauri::command]
pub fn katalog_formulierungen(kategorie_id: i64, state: tauri::State<AppState>) -> AppResult<Vec<Formulierung>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    katalog::list_formulierungen(&conn, kategorie_id)
}

#[tauri::command]
pub fn katalog_fach_anlegen(schuljahr_id: i64, name: String, state: tauri::State<AppState>) -> AppResult<i64> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::upsert_fach(&conn, schuljahr_id, &name, true)
}

#[tauri::command]
pub fn katalog_seed_default_faecher(schuljahr_id: i64, state: tauri::State<AppState>) -> AppResult<SeedSummary> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::seed_default_faecher(&conn, schuljahr_id)
}

#[tauri::command]
pub fn katalog_seed_default_floskeln(schuljahr_id: i64, state: tauri::State<AppState>) -> AppResult<FloskelnSeedSummary> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::seed_default_floskeln(&conn, schuljahr_id)
}

#[tauri::command]
pub fn katalog_kategorie_anlegen(schuljahr_id: i64, name: String, state: tauri::State<AppState>) -> AppResult<i64> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::upsert_kategorie(&conn, schuljahr_id, &name)
}

#[tauri::command]
pub fn katalog_formulierung_anlegen(kategorie_id: i64, text: String, state: tauri::State<AppState>) -> AppResult<i64> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::upsert_formulierung(&conn, kategorie_id, &text)
}

#[tauri::command]
pub fn katalog_fach_aktiv(id: i64, aktiv: bool, state: tauri::State<AppState>) -> AppResult<()> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::set_fach_aktiv(&conn, id, aktiv)
}

#[tauri::command]
pub fn katalog_kategorie_aktiv(id: i64, aktiv: bool, state: tauri::State<AppState>) -> AppResult<()> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::set_kategorie_aktiv(&conn, id, aktiv)
}

#[tauri::command]
pub fn katalog_formulierung_aktiv(id: i64, aktiv: bool, state: tauri::State<AppState>) -> AppResult<()> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::set_formulierung_aktiv(&conn, id, aktiv)
}

#[tauri::command]
pub fn katalog_fach_reihenfolge(id: i64, reihenfolge: i64, state: tauri::State<AppState>) -> AppResult<()> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::set_fach_reihenfolge(&conn, id, reihenfolge)
}

#[tauri::command]
pub fn katalog_kategorie_reihenfolge(id: i64, reihenfolge: i64, state: tauri::State<AppState>) -> AppResult<()> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::set_kategorie_reihenfolge(&conn, id, reihenfolge)
}

#[tauri::command]
pub fn katalog_formulierung_reihenfolge(id: i64, reihenfolge: i64, state: tauri::State<AppState>) -> AppResult<()> {
    require_admin(&state)?;
    let conn = open_db(&state)?;
    katalog::set_formulierung_reihenfolge(&conn, id, reihenfolge)
}

// --- Bewertung + Bemerkung ---

use crate::bewertung::{self, BewertungUpdate, MatrixZelle, SetResult};
use crate::bemerkung;

#[tauri::command]
pub fn bewertung_matrix(klasse_id: i64, fach_id: i64, state: tauri::State<AppState>) -> AppResult<Vec<MatrixZelle>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    bewertung::matrix(&conn, klasse_id, fach_id)
}

#[tauri::command]
pub fn bewertung_letzter_editor(
    klasse_id: i64,
    fach_id: i64,
    state: tauri::State<AppState>,
) -> AppResult<Option<(String, String)>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    bewertung::letzter_editor(&conn, klasse_id, fach_id)
}

#[tauri::command]
pub fn bewertung_set(update: BewertungUpdate, state: tauri::State<AppState>) -> AppResult<SetResult> {
    require_lehrer(&state)?;
    let mut conn = open_db(&state)?;
    bewertung::set(&mut conn, update)
}

#[tauri::command]
pub fn bewertung_wuerfeln(klasse_id: i64, state: tauri::State<AppState>) -> AppResult<(usize, usize)> {
    require_admin(&state)?;
    let mut conn = open_db(&state)?;
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    bewertung::wuerfle_klasse(&mut conn, klasse_id, 0.1, seed)
}

#[tauri::command]
pub fn bemerkung_get(schueler_id: i64, state: tauri::State<AppState>) -> AppResult<Option<(String, String, Option<String>)>> {
    require_klassenlehrer_oder_admin(&state)?;
    let conn = open_db(&state)?;
    bemerkung::get(&conn, schueler_id)
}

#[tauri::command]
pub fn bemerkung_set(
    schueler_id: i64,
    text: String,
    vorheriger_stand: Option<String>,
    editor_kuerzel: Option<String>,
    state: tauri::State<AppState>,
) -> AppResult<SetResult> {
    require_klassenlehrer_oder_admin(&state)?;
    let mut conn = open_db(&state)?;
    bemerkung::set(&mut conn, schueler_id, &text, vorheriger_stand, editor_kuerzel)
}

// --- Klassen + Schüler Mini-Reads ---

#[tauri::command]
pub fn klassenraum_klassen(schuljahr_id: i64, state: tauri::State<AppState>) -> AppResult<Vec<Klasse>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    stammdaten::list_klassen(&conn, schuljahr_id)
}

#[tauri::command]
pub fn klassenraum_schueler(klasse_id: i64, state: tauri::State<AppState>) -> AppResult<Vec<SchuelerMini>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    stammdaten::list_schueler(&conn, klasse_id)
}

// --- Modul-Uebersicht (read-only Aggregat pro Schueler:in) ---

use crate::uebersicht::{self, SchuelerUebersicht, VollstReport};

#[tauri::command]
pub fn uebersicht_schueler(schueler_id: i64, state: tauri::State<AppState>) -> AppResult<SchuelerUebersicht> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    uebersicht::schueler_uebersicht(&conn, schueler_id)
}

#[tauri::command]
pub fn uebersicht_klasse(klasse_id: i64, state: tauri::State<AppState>) -> AppResult<Vec<SchuelerUebersicht>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    uebersicht::klassen_uebersicht(&conn, klasse_id)
}

#[tauri::command]
pub fn uebersicht_vollstaendigkeit(klasse_id: i64, state: tauri::State<AppState>) -> AppResult<VollstReport> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    uebersicht::vollstaendigkeit_klasse(&conn, klasse_id)
}

// --- Goodies (Tageszitate) ---

use crate::goodies::{self, Zitat};

#[tauri::command]
pub fn goodies_zitat(state: tauri::State<AppState>) -> Option<Zitat> {
    let cfg = state.config.lock().unwrap();
    if !cfg.goodies.enabled {
        return None;
    }
    Some(goodies::aktuelles_zitat())
}

use crate::bug_report::{self, IssueResponse};

#[tauri::command]
pub fn bug_report_submit(
    titel: String,
    body: String,
    state: tauri::State<AppState>,
) -> AppResult<IssueResponse> {
    require_lehrer(&state)?;
    let cfg = state.config.lock().unwrap().clone();
    bug_report::submit(&cfg, &titel, &body)
}
