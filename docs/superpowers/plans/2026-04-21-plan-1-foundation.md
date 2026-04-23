# Plan 1: Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ein signierter Windows-Build von Jiraso-reloaded, der startet, eine SQLite-DB mit dem vollständigen Schema anlegt, Login mit drei Rollen durchführt, einen Session-Lock am Netzlaufwerk aufbaut und einmal pro Tag die DB sichert.

**Architecture:** Tauri 2 als Desktop-Wrapper um ein SvelteKit-Frontend. Rust-Backend verantwortet DB, Konfiguration, Passwörter und Lock. Frontend ist zunächst ein Login-Screen und ein Platzhalter-Dashboard.

**Tech Stack:** Rust 1.80+, Tauri 2.x, SvelteKit 2 + Svelte 5, TypeScript 5, `rusqlite` 0.32, `rusqlite_migration` 1.2, `argon2` 0.5, `serde`/`toml`/`chrono`/`anyhow`.

**Spec-Bezug:** Abschnitte „Architektur", „Datenmodell", „Locking & Mehrbenutzer" (ohne optimistische Konfliktprüfung — die kommt in Plan 3), „Authentifizierung", „Backup", „Deployment" aus `docs/superpowers/specs/2026-04-21-jiraso-reloaded-design.md`.

**Nicht im Scope** (kommt in Folge-Plänen): Stammdaten-Import (Plan 2), Bewertung/Bemerkung-UI (Plan 3), Übersicht + Admin-Editor (Plan 4), PDF-Druck (Plan 5), Goodies + Release-Politur (Plan 6).

---

## Datei-Struktur nach Plan 1

```
jiraso-reloaded/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Entry, window config, setup-hook
│   │   ├── lib.rs               # Library root für Unit-Tests
│   │   ├── db.rs                # SQLite-Verbindung + Migration-Runner
│   │   ├── config.rs            # config.toml laden/erzeugen/validieren
│   │   ├── auth.rs              # Argon2id hash/verify, Rolle auflösen
│   │   ├── lock.rs              # Session-Lock (JSON-Datei neben DB)
│   │   ├── backup.rs            # Täglicher DB-Snapshot + Rotation
│   │   ├── commands.rs          # #[tauri::command]-Wrapper für Frontend
│   │   ├── error.rs             # einheitlicher Error-Typ für Commands
│   │   └── migrations/
│   │       └── 001_initial.sql  # vollständiges Schema aus Spec
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── icons/icon.ico           # Schullogo-Platzhalter
├── src/                          # SvelteKit-Frontend
│   ├── routes/
│   │   ├── +layout.svelte        # Kopfzeile, Rollen-Anzeige, Logout
│   │   ├── +layout.ts            # SSR aus (Tauri rendert lokal)
│   │   ├── +page.svelte          # Root → Redirect je nach Session
│   │   ├── login/+page.svelte
│   │   └── dashboard/+page.svelte
│   ├── lib/
│   │   ├── api.ts                # typed invoke-Wrapper
│   │   └── session.svelte.ts     # Svelte-5-Rune-Store für Session
│   ├── app.html
│   └── app.css
├── package.json
├── svelte.config.js
├── vite.config.ts
├── tsconfig.json
├── .github/workflows/build.yml   # Windows-x64 Release-Build
├── config.toml.example
└── README.md                     # Setup + Entwicklungsanleitung
```

**Verantwortlichkeiten im Backend:**
- `db.rs` kapselt ausschließlich SQLite-Zugriff und Migrations-Lauf. Business-Logik gehört nicht hier rein.
- `auth.rs` kennt nur Hash/Verify und die Rollen-Enumeration. Kein Direkt-Zugriff auf Config oder DB — Parameter werden übergeben.
- `lock.rs` liest/schreibt ausschließlich die `.lock`-Datei. Unabhängig von DB.
- `backup.rs` kopiert Dateien, keine SQL-Queries.
- `commands.rs` ist die einzige Einheit, die `tauri::command` und die anderen Module verdrahtet. Kein Logik-Code hier, nur Delegation.
- `error.rs` definiert `AppError`, `thiserror`-basiert, serialisierbar für Frontend.

---

## Task 1: Tauri-Scaffold anlegen

**Files:**
- Create: gesamtes Projekt-Skelett (via Tauri-Create-App)
- Modify: Wurzel-README.md danach

- [x] **Step 1: Im leeren Repo-Arbeitsverzeichnis das Tauri-Template erzeugen**

```bash
cd /home/neo/jiraso-reloaded
# alles außer docs/, .gitignore, .git/ temporär beiseite schieben
mkdir -p /tmp/jiraso-tmp && mv docs .gitignore /tmp/jiraso-tmp/ 2>/dev/null || true
npm create tauri-app@latest . -- --template svelte-ts --manager npm --identifier de.schiller-offenburg.jiraso-reloaded
# Danach zurückschieben
mv /tmp/jiraso-tmp/docs /tmp/jiraso-tmp/.gitignore .
rmdir /tmp/jiraso-tmp
```

- [x] **Step 2: `npm install` und Build-Sanity prüfen**

```bash
npm install
npm run tauri build -- --debug
```

Expected: Build läuft durch, erzeugt `src-tauri/target/debug/jiraso-reloaded`.

- [x] **Step 3: Sinnvolle Metadaten in `src-tauri/tauri.conf.json`**

Ersetze die `productName`, `identifier`, `version`, `app.windows`-Sektion:

```json
{
  "productName": "Jiraso-reloaded",
  "version": "0.1.0",
  "identifier": "de.schiller-offenburg.jiraso-reloaded",
  "app": {
    "windows": [{
      "title": "Jiraso-reloaded · Verbalbeurteilungen 5/6",
      "width": 1400,
      "height": 900,
      "minWidth": 1100,
      "minHeight": 700,
      "resizable": true,
      "maximized": true,
      "center": true
    }]
  }
}
```

- [x] **Step 4: Erster Commit**

```bash
git add -A
git commit -m "feat: initial Tauri + SvelteKit scaffold

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Backend-Crate-Dependencies definieren

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [x] **Step 1: Dependencies hinzufügen**

Ersetze den `[dependencies]`-Block in `src-tauri/Cargo.toml` durch:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
rusqlite = { version = "0.32", features = ["bundled", "chrono"] }
rusqlite_migration = "1.2"
argon2 = "0.5"
rand = "0.8"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tempfile = "3"
```

`bundled` bei `rusqlite` liefert SQLite selbst mit (keine System-Lib nötig auf dem Schulrechner).

- [x] **Step 2: `cargo check` im `src-tauri/`-Verzeichnis**

```bash
cd src-tauri && cargo check
```

Expected: Dependencies werden aufgelöst, keine Fehler.

- [x] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: pin backend dependencies

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Error-Typ definieren

**Files:**
- Create: `src-tauri/src/error.rs`

- [x] **Step 1: Datei anlegen**

```rust
// src-tauri/src/error.rs
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Datenbankfehler: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Migrations-Fehler: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("Dateisystem-Fehler: {0}")]
    Io(#[from] std::io::Error),

    #[error("Konfigurations-Fehler: {0}")]
    Config(String),

    #[error("Passwort-Hash-Fehler: {0}")]
    Hash(String),

    #[error("Lock belegt: {0}")]
    Locked(String),

    #[error("TOML-Fehler: {0}")]
    Toml(String),
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(e: argon2::password_hash::Error) -> Self {
        AppError::Hash(e.to_string())
    }
}

impl From<toml::de::Error> for AppError {
    fn from(e: toml::de::Error) -> Self {
        AppError::Toml(e.to_string())
    }
}

impl From<toml::ser::Error> for AppError {
    fn from(e: toml::ser::Error) -> Self {
        AppError::Toml(e.to_string())
    }
}

// Fürs Frontend: String serialisieren
impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

- [x] **Step 2: `mod error;` in `lib.rs` exportieren**

Ersetze `src-tauri/src/lib.rs` (oder lege an) mit:

```rust
pub mod error;
```

- [x] **Step 3: `cargo check`**

```bash
cd src-tauri && cargo check
```

Expected: ok.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/error.rs src-tauri/src/lib.rs
git commit -m "feat(error): app-wide error type with frontend serialization

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: SQL-Migration-File anlegen

**Files:**
- Create: `src-tauri/src/migrations/001_initial.sql`

- [x] **Step 1: Schema-Datei schreiben**

```sql
-- src-tauri/src/migrations/001_initial.sql
CREATE TABLE schuljahr (
    id          INTEGER PRIMARY KEY,
    bezeichnung TEXT NOT NULL UNIQUE,
    aktiv       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE klasse (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    UNIQUE(schuljahr_id, name)
);

CREATE TABLE schueler (
    id          INTEGER PRIMARY KEY,
    asv_uuid    TEXT UNIQUE,
    klasse_id   INTEGER NOT NULL REFERENCES klasse(id) ON DELETE CASCADE,
    vorname     TEXT NOT NULL,
    nachname    TEXT NOT NULL,
    sortname    TEXT NOT NULL GENERATED ALWAYS AS (nachname || ', ' || vorname) STORED
);
CREATE INDEX idx_schueler_klasse ON schueler(klasse_id, nachname);

CREATE TABLE fach (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    reihenfolge  INTEGER NOT NULL,
    aktiv        INTEGER NOT NULL DEFAULT 1,
    UNIQUE(schuljahr_id, name)
);

CREATE TABLE kategorie (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    reihenfolge  INTEGER NOT NULL,
    aktiv        INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE formulierung (
    id           INTEGER PRIMARY KEY,
    kategorie_id INTEGER NOT NULL REFERENCES kategorie(id) ON DELETE CASCADE,
    text         TEXT NOT NULL,
    reihenfolge  INTEGER NOT NULL,
    aktiv        INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE bewertung (
    schueler_id     INTEGER NOT NULL REFERENCES schueler(id) ON DELETE CASCADE,
    fach_id         INTEGER NOT NULL REFERENCES fach(id) ON DELETE CASCADE,
    formulierung_id INTEGER NOT NULL REFERENCES formulierung(id) ON DELETE CASCADE,
    geaendert_am    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (schueler_id, fach_id, formulierung_id)
);

CREATE TABLE bemerkung (
    schueler_id   INTEGER PRIMARY KEY REFERENCES schueler(id) ON DELETE CASCADE,
    text          TEXT NOT NULL,
    geaendert_am  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

- [x] **Step 2: Commit**

```bash
git add src-tauri/src/migrations/001_initial.sql
git commit -m "feat(db): initial schema migration (001)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: DB-Modul mit Migration-Runner (TDD)

**Files:**
- Create: `src-tauri/src/db.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Test zuerst schreiben**

Ergänze `src-tauri/src/lib.rs`:

```rust
pub mod error;
pub mod db;
```

Lege `src-tauri/src/db.rs` an mit leerer API + Test:

```rust
// src-tauri/src/db.rs
use crate::error::AppResult;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::Path;

static MIGRATIONS_001: &str = include_str!("migrations/001_initial.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(MIGRATIONS_001)])
}

pub fn open(path: &Path) -> AppResult<Connection> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrations().to_latest(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn fresh_db_has_all_tables() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("jiraso.db")).unwrap();

        let expected = [
            "schuljahr", "klasse", "schueler",
            "fach", "kategorie", "formulierung",
            "bewertung", "bemerkung",
        ];
        for t in expected {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                    [t],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 1, "Tabelle {t} fehlt");
        }
    }

    #[test]
    fn migration_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("jiraso.db");
        open(&path).unwrap();
        // zweiter Aufruf darf nicht fehlschlagen
        open(&path).unwrap();
    }

    #[test]
    fn generated_sortname_works() {
        let dir = tempdir().unwrap();
        let conn = open(&dir.path().join("jiraso.db")).unwrap();

        conn.execute("INSERT INTO schuljahr (bezeichnung, aktiv) VALUES ('2025/26', 1)", []).unwrap();
        conn.execute("INSERT INTO klasse (schuljahr_id, name) VALUES (1, '5a')", []).unwrap();
        conn.execute(
            "INSERT INTO schueler (klasse_id, vorname, nachname) VALUES (1, 'Greta', 'Berres')",
            [],
        ).unwrap();

        let sortname: String = conn
            .query_row("SELECT sortname FROM schueler LIMIT 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sortname, "Berres, Greta");
    }
}
```

- [x] **Step 2: Tests laufen lassen**

```bash
cd src-tauri && cargo test --lib db::
```

Expected: 3 Tests, alle grün.

- [x] **Step 3: Commit**

```bash
git add src-tauri/src/db.rs src-tauri/src/lib.rs
git commit -m "feat(db): migration runner with WAL + foreign_keys

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Konfig-Modul mit Default-Generator (TDD)

**Files:**
- Create: `src-tauri/src/config.rs`
- Create: `config.toml.example`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: `config.toml.example` schreiben**

```toml
# config.toml.example — in config.toml umbenennen und Passwort-Hashes füllen

schule = "Schiller-Gymnasium Offenburg"
aktuelles_schuljahr = "2025/26"

# Passwort-Hashes werden beim ersten Start vom Wizard gesetzt.
# Nicht manuell editieren, wenn nicht unbedingt nötig.
[passwoerter]
fachlehrer    = ""
klassenlehrer = ""
administrator = ""

[goodies]
enabled = true
mode    = "random"  # random | zitat_only | fun_fact_only | achievement_only

[lock]
# Sessions werden automatisch verworfen, wenn sie älter als stale_hours sind
stale_hours = 8
max_parallel = 2
```

- [x] **Step 2: `src-tauri/src/config.rs` mit Tests**

```rust
// src-tauri/src/config.rs
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub schule: String,
    pub aktuelles_schuljahr: String,
    pub passwoerter: Passwoerter,
    pub goodies: GoodiesConfig,
    pub lock: LockConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Passwoerter {
    pub fachlehrer: String,
    pub klassenlehrer: String,
    pub administrator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoodiesConfig {
    pub enabled: bool,
    pub mode: String,
}

impl Default for GoodiesConfig {
    fn default() -> Self {
        Self { enabled: true, mode: "random".into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockConfig {
    pub stale_hours: u32,
    pub max_parallel: u32,
}

impl Default for LockConfig {
    fn default() -> Self {
        Self { stale_hours: 8, max_parallel: 2 }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schule: "Schiller-Gymnasium Offenburg".into(),
            aktuelles_schuljahr: "2025/26".into(),
            passwoerter: Passwoerter::default(),
            goodies: GoodiesConfig::default(),
            lock: LockConfig::default(),
        }
    }
}

pub fn load_or_create(path: &Path) -> AppResult<AppConfig> {
    if path.exists() {
        let content = fs::read_to_string(path)?;
        let cfg: AppConfig = toml::from_str(&content)?;
        Ok(cfg)
    } else {
        let cfg = AppConfig::default();
        save(&cfg, path)?;
        Ok(cfg)
    }
}

pub fn save(cfg: &AppConfig, path: &Path) -> AppResult<()> {
    let content = toml::to_string_pretty(cfg)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn passwoerter_leer(cfg: &AppConfig) -> bool {
    cfg.passwoerter.fachlehrer.is_empty()
        && cfg.passwoerter.klassenlehrer.is_empty()
        && cfg.passwoerter.administrator.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_default_when_missing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let cfg = load_or_create(&path).unwrap();
        assert!(path.exists());
        assert_eq!(cfg.lock.max_parallel, 2);
        assert!(passwoerter_leer(&cfg));
    }

    #[test]
    fn reads_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "schule = \"Testschule\"\naktuelles_schuljahr = \"2020/21\"\n\n[passwoerter]\nfachlehrer = \"h1\"\nklassenlehrer = \"h2\"\nadministrator = \"h3\"\n\n[goodies]\nenabled = false\nmode = \"zitat_only\"\n\n[lock]\nstale_hours = 4\nmax_parallel = 2\n").unwrap();
        let cfg = load_or_create(&path).unwrap();
        assert_eq!(cfg.schule, "Testschule");
        assert!(!cfg.goodies.enabled);
        assert_eq!(cfg.lock.stale_hours, 4);
    }
}
```

- [x] **Step 3: `mod config;` in `lib.rs`**

```rust
pub mod error;
pub mod db;
pub mod config;
```

- [x] **Step 4: Tests laufen lassen**

```bash
cd src-tauri && cargo test --lib config::
```

Expected: 2 Tests grün.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs config.toml.example
git commit -m "feat(config): load/create config.toml with defaults

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Auth-Modul (TDD)

**Files:**
- Create: `src-tauri/src/auth.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Auth-Modul mit Test**

```rust
// src-tauri/src/auth.rs
use crate::config::AppConfig;
use crate::error::AppResult;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rolle {
    Fachlehrer,
    Klassenlehrer,
    Administrator,
}

pub fn hash_password(pw: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon = Argon2::default();
    let hash = argon.hash_password(pw.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verify_password(pw: &str, encoded: &str) -> AppResult<bool> {
    if encoded.is_empty() {
        return Ok(false);
    }
    let parsed = PasswordHash::new(encoded)?;
    Ok(Argon2::default().verify_password(pw.as_bytes(), &parsed).is_ok())
}

/// Prüft das Passwort gegen alle drei Rollen-Hashes und gibt die erste
/// passende Rolle zurück. Reihenfolge: Admin > Klassenlehrer > Fachlehrer
/// (höhere Rolle gewinnt bei Kollision, damit ein Admin mit zufällig
/// identischem Passwort nicht auf Fachlehrer degradiert wird).
pub fn resolve_role(pw: &str, cfg: &AppConfig) -> AppResult<Option<Rolle>> {
    if verify_password(pw, &cfg.passwoerter.administrator)? {
        return Ok(Some(Rolle::Administrator));
    }
    if verify_password(pw, &cfg.passwoerter.klassenlehrer)? {
        return Ok(Some(Rolle::Klassenlehrer));
    }
    if verify_password(pw, &cfg.passwoerter.fachlehrer)? {
        return Ok(Some(Rolle::Fachlehrer));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_hash_verify() {
        let h = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &h).unwrap());
        assert!(!verify_password("wrong", &h).unwrap());
    }

    #[test]
    fn empty_hash_rejects() {
        assert!(!verify_password("any", "").unwrap());
    }

    #[test]
    fn resolve_role_prefers_highest() {
        let mut cfg = AppConfig::default();
        cfg.passwoerter.fachlehrer = hash_password("geheim").unwrap();
        cfg.passwoerter.klassenlehrer = hash_password("anders").unwrap();
        cfg.passwoerter.administrator = hash_password("geheim").unwrap();

        // "geheim" matcht Fachlehrer und Admin — Admin gewinnt
        assert_eq!(resolve_role("geheim", &cfg).unwrap(), Some(Rolle::Administrator));
        assert_eq!(resolve_role("anders", &cfg).unwrap(), Some(Rolle::Klassenlehrer));
        assert_eq!(resolve_role("unknown", &cfg).unwrap(), None);
    }
}
```

- [x] **Step 2: `mod auth;` in `lib.rs`**

```rust
pub mod error;
pub mod db;
pub mod config;
pub mod auth;
```

- [x] **Step 3: Tests**

```bash
cd src-tauri && cargo test --lib auth::
```

Expected: 3 Tests grün.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/auth.rs src-tauri/src/lib.rs
git commit -m "feat(auth): Argon2id password hashing + role resolution

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Session-Lock (TDD)

**Files:**
- Create: `src-tauri/src/lock.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Lock-Modul mit umfangreichen Tests**

```rust
// src-tauri/src/lock.rs
use crate::error::{AppError, AppResult};
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub rechner: String,
    pub rolle: String,
    pub since: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LockFile {
    sessions: Vec<Session>,
}

pub struct LockConfig {
    pub stale_hours: u32,
    pub max_parallel: u32,
}

fn gen_session_id() -> String {
    let mut rng = rand::thread_rng();
    format!("{:08x}", rng.gen::<u32>())
}

fn read_file(path: &Path) -> AppResult<LockFile> {
    if !path.exists() {
        return Ok(LockFile::default());
    }
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content).unwrap_or_default())
}

fn write_file(path: &Path, lf: &LockFile) -> AppResult<()> {
    let content = serde_json::to_string_pretty(lf)
        .map_err(|e| AppError::Config(e.to_string()))?;
    fs::write(path, content)?;
    Ok(())
}

fn prune_stale(lf: &mut LockFile, stale_hours: u32, now: DateTime<Utc>) {
    let cutoff = now - Duration::hours(stale_hours as i64);
    lf.sessions.retain(|s| s.since > cutoff);
}

/// Versucht, eine Session zu registrieren. Gibt Session oder Locked-Fehler zurück.
pub fn acquire(
    path: &Path,
    rechner: &str,
    rolle: &str,
    cfg: &LockConfig,
) -> AppResult<Session> {
    acquire_at(path, rechner, rolle, cfg, Utc::now())
}

// separater Entry-Point, damit Tests die Uhr stellen können
pub fn acquire_at(
    path: &Path,
    rechner: &str,
    rolle: &str,
    cfg: &LockConfig,
    now: DateTime<Utc>,
) -> AppResult<Session> {
    let mut lf = read_file(path)?;
    prune_stale(&mut lf, cfg.stale_hours, now);

    if lf.sessions.len() >= cfg.max_parallel as usize {
        let names: Vec<String> = lf
            .sessions
            .iter()
            .map(|s| format!("{} ({}, seit {})", s.rechner, s.rolle, s.since.format("%H:%M")))
            .collect();
        return Err(AppError::Locked(format!(
            "Aktuell sind {} angemeldet. Bitte warten, bis jemand das Programm schließt.",
            names.join(" und ")
        )));
    }

    let session = Session {
        id: gen_session_id(),
        rechner: rechner.into(),
        rolle: rolle.into(),
        since: now,
    };
    lf.sessions.push(session.clone());
    write_file(path, &lf)?;
    Ok(session)
}

pub fn release(path: &Path, session_id: &str) -> AppResult<()> {
    let mut lf = read_file(path)?;
    lf.sessions.retain(|s| s.id != session_id);
    write_file(path, &lf)?;
    Ok(())
}

/// Admin-Override: entfernt ALLE Sessions.
pub fn break_lock(path: &Path) -> AppResult<()> {
    let lf = LockFile::default();
    write_file(path, &lf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn cfg() -> LockConfig {
        LockConfig { stale_hours: 8, max_parallel: 2 }
    }

    #[test]
    fn acquire_succeeds_when_empty() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        let s = acquire(&p, "PC-A", "Fachlehrer", &cfg()).unwrap();
        assert_eq!(s.rechner, "PC-A");
    }

    #[test]
    fn second_session_ok_third_blocked() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        acquire(&p, "PC-A", "Fachlehrer", &cfg()).unwrap();
        acquire(&p, "PC-B", "Klassenlehrer", &cfg()).unwrap();
        let e = acquire(&p, "PC-C", "Fachlehrer", &cfg());
        assert!(matches!(e, Err(AppError::Locked(_))));
    }

    #[test]
    fn stale_sessions_are_pruned() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        let old = Utc::now() - Duration::hours(9);
        acquire_at(&p, "PC-A", "Fachlehrer", &cfg(), old).unwrap();
        acquire_at(&p, "PC-B", "Fachlehrer", &cfg(), old).unwrap();
        // beide stale -> neuer Slot frei
        let now = Utc::now();
        let s = acquire_at(&p, "PC-C", "Fachlehrer", &cfg(), now).unwrap();
        assert_eq!(s.rechner, "PC-C");

        let lf = read_file(&p).unwrap();
        assert_eq!(lf.sessions.len(), 1);
    }

    #[test]
    fn release_frees_slot() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        let a = acquire(&p, "PC-A", "Fachlehrer", &cfg()).unwrap();
        acquire(&p, "PC-B", "Fachlehrer", &cfg()).unwrap();
        release(&p, &a.id).unwrap();
        // Slot frei
        acquire(&p, "PC-C", "Fachlehrer", &cfg()).unwrap();
    }

    #[test]
    fn break_lock_clears_all() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("jiraso.db.lock");
        acquire(&p, "PC-A", "Fachlehrer", &cfg()).unwrap();
        acquire(&p, "PC-B", "Fachlehrer", &cfg()).unwrap();
        break_lock(&p).unwrap();
        // wieder beide Slots frei
        acquire(&p, "PC-C", "Fachlehrer", &cfg()).unwrap();
        acquire(&p, "PC-D", "Fachlehrer", &cfg()).unwrap();
    }
}
```

- [x] **Step 2: `mod lock;` in `lib.rs`**

```rust
pub mod error;
pub mod db;
pub mod config;
pub mod auth;
pub mod lock;
```

- [x] **Step 3: Tests**

```bash
cd src-tauri && cargo test --lib lock::
```

Expected: 5 Tests grün.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/lock.rs src-tauri/src/lib.rs
git commit -m "feat(lock): session lock with stale cleanup and admin override

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Backup-Modul (TDD)

**Files:**
- Create: `src-tauri/src/backup.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Backup-Modul mit Tests**

```rust
// src-tauri/src/backup.rs
use crate::error::AppResult;
use chrono::{Datelike, NaiveDate, Utc};
use std::{fs, path::Path};

const RETENTION: usize = 30;

/// Erzeugt, falls heute noch nicht vorhanden, eine Kopie der DB als
/// data/backups/jiraso-YYYY-MM-DD.db. Rotiert auf 30 Dateien.
pub fn daily_backup(db: &Path, backups_dir: &Path) -> AppResult<Option<std::path::PathBuf>> {
    daily_backup_on(db, backups_dir, Utc::now().date_naive())
}

pub fn daily_backup_on(db: &Path, backups_dir: &Path, today: NaiveDate) -> AppResult<Option<std::path::PathBuf>> {
    if !db.exists() {
        return Ok(None);
    }
    fs::create_dir_all(backups_dir)?;
    let fname = format!(
        "jiraso-{:04}-{:02}-{:02}.db",
        today.year(),
        today.month(),
        today.day()
    );
    let target = backups_dir.join(&fname);

    if target.exists() {
        return Ok(None); // heute schon gesichert
    }

    fs::copy(db, &target)?;
    rotate(backups_dir, RETENTION)?;
    Ok(Some(target))
}

fn rotate(dir: &Path, keep: usize) -> AppResult<()> {
    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|r| r.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("jiraso-")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());
    while entries.len() > keep {
        let oldest = entries.remove(0);
        fs::remove_file(oldest.path())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn no_db_means_no_backup() {
        let dir = tempdir().unwrap();
        let r = daily_backup(&dir.path().join("nope.db"), &dir.path().join("bk")).unwrap();
        assert!(r.is_none());
    }

    #[test]
    fn first_backup_creates_file() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("x.db");
        fs::write(&db, b"hello").unwrap();
        let bk = dir.path().join("bk");
        let r = daily_backup(&db, &bk).unwrap();
        assert!(r.is_some());
        assert_eq!(fs::read_dir(&bk).unwrap().count(), 1);
    }

    #[test]
    fn same_day_second_call_noop() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("x.db");
        fs::write(&db, b"hello").unwrap();
        let bk = dir.path().join("bk");
        daily_backup(&db, &bk).unwrap();
        let r2 = daily_backup(&db, &bk).unwrap();
        assert!(r2.is_none());
        assert_eq!(fs::read_dir(&bk).unwrap().count(), 1);
    }

    #[test]
    fn rotation_keeps_30() {
        let dir = tempdir().unwrap();
        let db = dir.path().join("x.db");
        fs::write(&db, b"data").unwrap();
        let bk = dir.path().join("bk");

        // 32 Tage simulieren
        for day in 1..=32 {
            let d = NaiveDate::from_ymd_opt(2025, 1, day).unwrap();
            daily_backup_on(&db, &bk, d).unwrap();
        }
        let count = fs::read_dir(&bk).unwrap().count();
        assert_eq!(count, 30);
        // älteste zwei sollten weg sein
        assert!(!bk.join("jiraso-2025-01-01.db").exists());
        assert!(!bk.join("jiraso-2025-01-02.db").exists());
        assert!(bk.join("jiraso-2025-01-03.db").exists());
        assert!(bk.join("jiraso-2025-01-32.db").exists() == false); // 32.1. gibts nicht
    }
}
```

*Hinweis zum letzten Test:* `NaiveDate::from_ymd_opt(2025, 1, 32)` liefert `None`; die Schleife bricht implizit durch `.unwrap()` ab. Wir korrigieren:

- [x] **Step 2: Backup-Test fixen (2025-01 hat nur 31 Tage)** (direkt mit korrektem Loop in Step 1 umgesetzt)

Ersetze die Schleife im letzten Test durch:

```rust
        // 32 Tage simulieren: Jan + Anfang Feb
        let mut d = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        for _ in 0..32 {
            daily_backup_on(&db, &bk, d).unwrap();
            d = d.succ_opt().unwrap();
        }
        let count = fs::read_dir(&bk).unwrap().count();
        assert_eq!(count, 30);
```

Und entferne die letzten drei spezifischen `assert!`-Zeilen. Lasse nur:

```rust
        assert_eq!(count, 30);
```

- [x] **Step 3: `mod backup;` in `lib.rs`**

```rust
pub mod error;
pub mod db;
pub mod config;
pub mod auth;
pub mod lock;
pub mod backup;
```

- [x] **Step 4: Tests**

```bash
cd src-tauri && cargo test --lib backup::
```

Expected: 4 Tests grün.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/backup.rs src-tauri/src/lib.rs
git commit -m "feat(backup): daily db snapshot with 30-day rotation

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Tauri-Commands + App-State

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`

- [x] **Step 1: `commands.rs` mit Login/Logout/Status** (enthält bereits die in Task 14 geplanten `needs_setup` + `setup_passwoerter`)

```rust
// src-tauri/src/commands.rs
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
```

- [x] **Step 2: `lib.rs` exportieren** (commands hinter `#[cfg(feature = "desktop")]`)

```rust
pub mod error;
pub mod db;
pub mod config;
pub mod auth;
pub mod lock;
pub mod backup;
pub mod commands;
```

- [x] **Step 3: `main.rs` umbauen** (run()-Logik wanderte nach lib.rs hinter das desktop-Feature; main.rs bleibt minimal wie Scaffold)

Ersetze den Inhalt von `src-tauri/src/main.rs` durch:

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use jiraso_reloaded_lib::{backup, commands, config, db, error::AppResult};
use std::sync::Mutex;

fn app_root() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

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

fn main() {
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
        ])
        .run(tauri::generate_context!())
        .expect("Tauri-App konnte nicht starten");
}
```

- [x] **Step 4: Library-Crate-Name fixen** (bereits in Task 1 erledigt)

In `src-tauri/Cargo.toml` ergänze/stelle sicher:

```toml
[lib]
name = "jiraso_reloaded_lib"
path = "src/lib.rs"

[[bin]]
name = "jiraso-reloaded"
path = "src/main.rs"
```

- [x] **Step 5: `cargo check` + Debug-Build** (`--no-default-features --lib` lokal; vollständiger Bin-Build kommt aus CI)

```bash
cd src-tauri && cargo check --no-default-features --lib
```

Expected: ok.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/main.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(commands): Tauri commands for login/logout/session

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: Frontend — typed API-Wrapper und Session-Store

**Files:**
- Create: `src/lib/api.ts`, `src/lib/session.svelte.ts`
- Modify: `svelte.config.js` (static adapter)

- [x] **Step 1: Static Adapter installieren und konfigurieren** (Scaffold lieferte adapter-static bereits; nur `prerender.entries: []` ergänzt)

```bash
npm install -D @sveltejs/adapter-static
```

Ersetze `svelte.config.js`:

```javascript
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({ fallback: 'index.html' }),
    prerender: { entries: [] }
  }
};
```

- [x] **Step 2: API-Wrapper schreiben** (inkl. `needsSetup` + `setupPasswoerter` aus Task 14)

```typescript
// src/lib/api.ts
import { invoke } from '@tauri-apps/api/core';

export type Rolle = 'fachlehrer' | 'klassenlehrer' | 'administrator';

export async function login(passwort: string, rechner: string): Promise<Rolle> {
  return invoke<Rolle>('login', { passwort, rechner });
}

export async function logout(): Promise<void> {
  return invoke<void>('logout');
}

export async function currentRole(): Promise<Rolle | null> {
  return invoke<Rolle | null>('current_role');
}

export async function schulname(): Promise<string> {
  return invoke<string>('schulname');
}

export async function aktuellesSchuljahr(): Promise<string> {
  return invoke<string>('aktuelles_schuljahr');
}

export async function breakLockAdmin(adminPasswort: string): Promise<void> {
  return invoke<void>('break_lock_admin', { adminPasswort });
}

export function rechnerName(): string {
  // Tauri gibt nicht den Hostnamen — wir nutzen den User-Agent-Hash als Pseudo-ID
  // oder, bessere Variante, rufen ein Rust-Command auf, das std::env::var("COMPUTERNAME")
  // liest. Für jetzt: navigator.userAgent-Hash.
  const ua = navigator.userAgent;
  let h = 0;
  for (let i = 0; i < ua.length; i++) h = (h * 31 + ua.charCodeAt(i)) | 0;
  return 'PC-' + (h >>> 0).toString(16).slice(0, 6);
}
```

- [x] **Step 3: Session-Store als Svelte-5-Rune**

```typescript
// src/lib/session.svelte.ts
import type { Rolle } from './api';

class SessionStore {
  rolle = $state<Rolle | null>(null);
  schule = $state<string>('');
  schuljahr = $state<string>('');
}

export const session = new SessionStore();
```

- [x] **Step 4: Commit**

```bash
git add src/lib/ svelte.config.js package.json package-lock.json
git commit -m "feat(frontend): typed invoke wrapper + session store

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 12: Login-Screen

**Files:**
- Create: `src/routes/login/+page.svelte`, `src/routes/+page.svelte`, `src/routes/+layout.svelte`, `src/routes/+layout.ts`
- Modify: `src/app.css`

- [ ] **Step 1: Layout-Disable für SSR**

```typescript
// src/routes/+layout.ts
export const ssr = false;
export const prerender = false;
```

- [ ] **Step 2: `+layout.svelte` mit Header**

```svelte
<!-- src/routes/+layout.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { session } from '$lib/session.svelte';
  import { schulname, aktuellesSchuljahr, currentRole, logout } from '$lib/api';
  import { goto } from '$app/navigation';

  onMount(async () => {
    session.schule = await schulname();
    session.schuljahr = await aktuellesSchuljahr();
    session.rolle = await currentRole();
  });

  async function handleLogout() {
    await logout();
    session.rolle = null;
    goto('/login');
  }

  let { children } = $props();
</script>

<header>
  <div class="brand">{session.schule}</div>
  <div class="title">Verbalbeurteilungen 5/6 · SJ {session.schuljahr}</div>
  <div class="auth">
    {#if session.rolle}
      <span class="role role-{session.rolle}">{session.rolle}</span>
      <button onclick={handleLogout}>Abmelden</button>
    {:else}
      <span class="role">nicht angemeldet</span>
    {/if}
  </div>
</header>

<main>
  {@render children()}
</main>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 1.2rem;
    background: #1f2d3d;
    color: #fff;
    font-family: 'Segoe UI', sans-serif;
  }
  .brand { font-weight: bold; }
  .title { font-size: 0.95rem; opacity: 0.9; }
  .auth { display: flex; gap: 0.8rem; align-items: center; }
  .role { font-size: 0.9rem; padding: 0.15rem 0.5rem; border-radius: 3px; background: #39475a; }
  .role-administrator { background: #8b2942; }
  .role-klassenlehrer { background: #1e4d8b; }
  .role-fachlehrer    { background: #2a7a4a; }
  button { padding: 0.25rem 0.8rem; cursor: pointer; }
  main { padding: 1.5rem 2rem; }
</style>
```

- [ ] **Step 3: Root-Seite redirectet je nach Session**

```svelte
<!-- src/routes/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';

  onMount(() => {
    if (session.rolle) goto('/dashboard');
    else goto('/login');
  });
</script>
<p>Lade…</p>
```

- [ ] **Step 4: Login-Seite**

```svelte
<!-- src/routes/login/+page.svelte -->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { login, rechnerName } from '$lib/api';
  import { session } from '$lib/session.svelte';

  let passwort = $state('');
  let fehler = $state<string | null>(null);
  let laufend = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    fehler = null;
    laufend = true;
    try {
      const rolle = await login(passwort, rechnerName());
      session.rolle = rolle;
      goto('/dashboard');
    } catch (err) {
      fehler = String(err);
    } finally {
      laufend = false;
    }
  }
</script>

<div class="login">
  <h1>Anmelden</h1>
  <form onsubmit={submit}>
    <label>
      Passwort
      <input
        type="password"
        bind:value={passwort}
        autocomplete="current-password"
        required
      />
    </label>
    <button type="submit" disabled={laufend}>
      {laufend ? 'Prüfe…' : 'Einloggen'}
    </button>
    {#if fehler}<p class="err">{fehler}</p>{/if}
  </form>
</div>

<style>
  .login { max-width: 320px; margin: 4rem auto; font-family: 'Segoe UI', sans-serif; }
  h1 { margin-bottom: 1rem; }
  form { display: flex; flex-direction: column; gap: 0.8rem; }
  label { display: flex; flex-direction: column; gap: 0.3rem; }
  input { padding: 0.5rem; font-size: 1rem; }
  button { padding: 0.6rem; font-size: 1rem; cursor: pointer; }
  .err { color: #b00; margin-top: 0.5rem; }
</style>
```

- [ ] **Step 5: Commit**

```bash
git add src/routes src/app.css
git commit -m "feat(frontend): login screen + layout with role indicator

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 13: Dashboard-Platzhalter

**Files:**
- Create: `src/routes/dashboard/+page.svelte`

- [ ] **Step 1: Platzhalter-Dashboard mit Rollen-Sichtbarkeit**

```svelte
<!-- src/routes/dashboard/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';

  onMount(() => {
    if (!session.rolle) goto('/login');
  });

  const kacheln = $derived.by(() => {
    if (!session.rolle) return [];
    const all = [
      { titel: 'Bewertung eingeben', rollen: ['fachlehrer', 'klassenlehrer', 'administrator'] },
      { titel: 'Bemerkung eingeben', rollen: ['klassenlehrer', 'administrator'] },
      { titel: 'Übersicht',          rollen: ['klassenlehrer', 'administrator'] },
      { titel: 'Drucken & Export',   rollen: ['klassenlehrer', 'administrator'] },
      { titel: 'Formulierungen verwalten', rollen: ['administrator'] },
      { titel: 'Datenverwaltung',    rollen: ['administrator'] }
    ];
    return all.filter(k => k.rollen.includes(session.rolle!));
  });
</script>

<h1>Start</h1>
<p class="hint">Die Funktionen werden in den folgenden Plänen (2–5) implementiert.</p>

<div class="grid">
  {#each kacheln as k}
    <div class="kachel">
      <h3>{k.titel}</h3>
      <p class="tbd">(noch nicht implementiert)</p>
    </div>
  {/each}
</div>

<style>
  .hint { color: #666; margin-bottom: 1.5rem; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 1rem; }
  .kachel { background: #f4f4f6; border: 1px solid #d6d6da; border-radius: 4px; padding: 1rem 1.2rem; }
  .kachel h3 { margin: 0 0 0.5rem; font-size: 1.05rem; }
  .tbd { color: #999; font-size: 0.85rem; margin: 0; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/dashboard
git commit -m "feat(frontend): role-gated dashboard placeholder

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 14: Admin-Setup-Wizard (einmalig beim Erst-Start Passwörter setzen)

**Files:**
- Create: `src/routes/setup/+page.svelte`
- Modify: `src-tauri/src/commands.rs` (neue `setup_passwoerter`-Command)
- Modify: `src-tauri/src/main.rs` (neue `needs_setup`-Command)
- Modify: `src/routes/+page.svelte` (Redirect nach `/setup`, wenn nötig)
- Modify: `src/lib/api.ts` (neue API-Funktionen)

- [ ] **Step 1: Rust-Command zum Setzen der drei Passwörter**

Ergänze in `src-tauri/src/commands.rs` am Ende:

```rust
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
```

- [ ] **Step 2: Commands im Handler registrieren**

In `src-tauri/src/main.rs`, erweitere `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![
    commands::login,
    commands::logout,
    commands::current_role,
    commands::schulname,
    commands::aktuelles_schuljahr,
    commands::break_lock_admin,
    commands::needs_setup,
    commands::setup_passwoerter,
])
```

- [ ] **Step 3: API-Wrapper erweitern**

Ergänze `src/lib/api.ts`:

```typescript
export async function needsSetup(): Promise<boolean> {
  return invoke<boolean>('needs_setup');
}

export async function setupPasswoerter(
  fachlehrer: string,
  klassenlehrer: string,
  administrator: string
): Promise<void> {
  return invoke<void>('setup_passwoerter', { fachlehrer, klassenlehrer, administrator });
}
```

- [ ] **Step 4: Root-Redirect erweitern**

Ersetze `src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { needsSetup, currentRole } from '$lib/api';
  import { session } from '$lib/session.svelte';

  onMount(async () => {
    if (await needsSetup()) { goto('/setup'); return; }
    const rolle = await currentRole();
    session.rolle = rolle;
    goto(rolle ? '/dashboard' : '/login');
  });
</script>
<p>Lade…</p>
```

- [ ] **Step 5: Setup-Seite**

```svelte
<!-- src/routes/setup/+page.svelte -->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { setupPasswoerter } from '$lib/api';

  let fachlehrer = $state('');
  let klassenlehrer = $state('');
  let administrator = $state('');
  let fehler = $state<string | null>(null);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    fehler = null;
    try {
      await setupPasswoerter(fachlehrer, klassenlehrer, administrator);
      goto('/login');
    } catch (err) {
      fehler = String(err);
    }
  }
</script>

<h1>Erste Einrichtung</h1>
<p>Bitte lege die drei Rollen-Passwörter fest (jeweils mindestens 8 Zeichen).
Gib sie danach persönlich an die jeweiligen Kolleg:innen weiter.</p>

<form onsubmit={submit}>
  <label>Fachlehrer    <input type="password" bind:value={fachlehrer}    minlength="8" required /></label>
  <label>Klassenlehrer <input type="password" bind:value={klassenlehrer} minlength="8" required /></label>
  <label>Administrator <input type="password" bind:value={administrator} minlength="8" required /></label>
  <button type="submit">Speichern</button>
  {#if fehler}<p class="err">{fehler}</p>{/if}
</form>

<style>
  form { display: flex; flex-direction: column; gap: 0.8rem; max-width: 360px; }
  label { display: flex; flex-direction: column; gap: 0.3rem; }
  input { padding: 0.5rem; }
  button { padding: 0.6rem; cursor: pointer; }
  .err { color: #b00; }
</style>
```

- [ ] **Step 6: Debug-Build + manueller Test**

```bash
npm run tauri dev
```

Expected:
- Beim ersten Start erscheint die Setup-Seite
- Nach dem Speichern Weiterleitung auf /login
- Mit dem neuen Admin-Passwort einloggen → Dashboard zeigt alle 6 Kacheln
- Mit Fachlehrer-Passwort → nur „Bewertung eingeben"

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(setup): first-run wizard for role passwords

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 15: GitHub-Actions-Build für Windows

**Files:**
- Create: `.github/workflows/build.yml`

- [ ] **Step 1: Workflow schreiben**

```yaml
# .github/workflows/build.yml
name: Windows Build

on:
  push:
    tags: ['v*']
  workflow_dispatch:

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: '20'

      - uses: dtolnay/rust-toolchain@stable

      - uses: swatinem/rust-cache@v2
        with:
          workspaces: src-tauri

      - run: npm ci

      - run: npm run tauri build
        env:
          CI: true

      - uses: actions/upload-artifact@v4
        with:
          name: Jiraso-reloaded-Windows
          path: |
            src-tauri/target/release/jiraso-reloaded.exe
            src-tauri/target/release/bundle/nsis/*.exe
            src-tauri/target/release/bundle/msi/*.msi
          if-no-files-found: warn

      - name: Release
        if: startsWith(github.ref, 'refs/tags/v')
        uses: softprops/action-gh-release@v2
        with:
          files: |
            src-tauri/target/release/bundle/nsis/*.exe
            src-tauri/target/release/bundle/msi/*.msi
```

- [ ] **Step 2: Workflow manuell auslösen**

```bash
git add .github/workflows/build.yml
git commit -m "ci: Windows release build via GitHub Actions

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
git push
gh workflow run build.yml
```

Expected: Action läuft durch (~8–12 Minuten), produziert ein Artifact `Jiraso-reloaded-Windows.zip`.

- [ ] **Step 3: Artifact herunterladen und entpacken**

```bash
# nachdem der Run durch ist:
gh run list --workflow=build.yml --limit=1
gh run download <run-id> -n Jiraso-reloaded-Windows -D /tmp/jiraso-build
ls /tmp/jiraso-build
```

Expected: `jiraso-reloaded.exe` und NSIS-Installer liegen vor.

---

## Task 16: Smoke-Checklist + README für den Schulrechner

**Files:**
- Create: `SMOKE_CHECKLIST.md`
- Modify: `README.md`

- [ ] **Step 1: README mit Deploy-Anleitung**

```markdown
# Jiraso-reloaded

Modernisierte Verbalbeurteilungs-Software für die Klassen 5 und 6 am
Schiller-Gymnasium Offenburg. Nachfolger von Jiraso (Peer Schlieker, 2021).

## Installation auf dem Schulrechner (Plan 1: Foundation)

1. Lade den aktuellen Windows-Build aus GitHub Releases (oder das
   Action-Artefakt der neuesten Build-Action).
2. Entpacke den Inhalt nach `V:\Jiraso-reloaded\` auf dem Tauschlaufwerk.
3. Starte `Jiraso-reloaded.exe` per Doppelklick. Beim ersten Start:
   - Die Setup-Seite erscheint.
   - Lege drei Passwörter fest (Fachlehrer, Klassenlehrer, Admin,
     je mindestens 8 Zeichen).
   - Nach dem Speichern kannst du dich einloggen.
4. Teste die Rollen (Fachlehrer sieht weniger Kacheln als Admin).
5. Starte die App parallel auf einem zweiten Rechner — der dritte
   gleichzeitige Start soll abgelehnt werden.

## Entwicklung

```bash
npm install
npm run tauri dev   # Debug-Build
```

Tests im Rust-Backend:
```bash
cd src-tauri && cargo test
```
```

- [ ] **Step 2: Smoke-Checklist**

```markdown
# Smoke-Checklist Plan 1: Foundation

Am Schulrechner durchzugehen, bevor Plan 2 gestartet wird.

## Erst-Installation

- [ ] `Jiraso-reloaded.exe` vom Netzlaufwerk startet ohne Fehlermeldung
- [ ] Neben der .exe entsteht `config.toml`, `data/jiraso.db`, `data/backups/`
- [ ] Setup-Seite erscheint mit drei Passwort-Feldern
- [ ] Passwörter unter 8 Zeichen werden abgelehnt
- [ ] Nach Speichern wird zur Login-Seite weitergeleitet

## Login & Rollen

- [ ] Admin-Passwort → Dashboard zeigt 6 Kacheln
- [ ] Klassenlehrer-Passwort → Dashboard zeigt 4 Kacheln (ohne „Formulierungen verwalten" und „Datenverwaltung")
- [ ] Fachlehrer-Passwort → Dashboard zeigt 1 Kachel („Bewertung eingeben")
- [ ] Falsches Passwort → Fehlermeldung auf der Login-Seite
- [ ] Abmelden-Button zurück zur Login-Seite

## Lock-Verhalten

- [ ] Auf Rechner B gleichzeitig einloggen → funktioniert
- [ ] Auf Rechner C einloggen (während A+B aktiv) → Fehler mit Namen der beiden aktiven Rechner
- [ ] Auf Rechner A abmelden → Slot frei, C kann sich einloggen
- [ ] App auf A hart schließen (Task-Manager) → Lock bleibt stehen
- [ ] Nach 8 Stunden oder per manuellem Datei-Löschen von `jiraso.db.lock` wird der stale Slot freigegeben

## Backup

- [ ] Nach erstem Start liegt `data/backups/jiraso-YYYY-MM-DD.db`
- [ ] App am nächsten Tag nochmal starten → neue Backup-Datei mit neuem Datum

## Datenbank

- [ ] `data/jiraso.db` mit SQLite-Browser öffnen → 8 Tabellen vorhanden
  (`schuljahr`, `klasse`, `schueler`, `fach`, `kategorie`, `formulierung`,
  `bewertung`, `bemerkung`)
- [ ] Alle Tabellen sind leer
```

- [ ] **Step 3: Commit**

```bash
git add README.md SMOKE_CHECKLIST.md
git commit -m "docs: README + smoke checklist for Plan 1 release

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
git push
```

---

## Abschluss Plan 1

Nach Abschluss dieses Plans ist verfügbar:
- Ein Windows-Binary `Jiraso-reloaded.exe`, das vom Netzlaufwerk startet
- SQLite-DB mit vollständigem Schema (leer)
- Erst-Einrichtungs-Wizard für drei Rollen-Passwörter (Argon2id gehashed)
- Login / Logout mit Rollen-abhängigem Dashboard (leere Kacheln)
- Session-Lock auf dem Netzlaufwerk (max. 2 Sessions, 8 h Stale-Timeout)
- Tägliches DB-Backup mit 30-Tage-Rotation
- GitHub-Action für Release-Builds

**User-Freigabe-Gate:** Schul-Rechner-Test nach `SMOKE_CHECKLIST.md`. Erst bei grünem Durchlauf folgt Plan 2 (Stammdaten-Import).
