# Plan 2: Stammdaten-Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Admin legt via UI ein Schuljahr an, wählt die `alleSchueler.xlsx` (ASV-BW-Export), sieht eine Vorschau der erkannten Spalten + Zeilen, bestätigt und bekommt die Schüler:innen/Klassen in die SQLite-DB geschrieben. Re-Import aus derselben Datei matched bestehende Einträge per ASV-UUID.

**Architecture:** Ein neues Rust-Modul `import.rs` kapselt XLSX-Parsing (`calamine`), heuristisches Spalten-Matching und Diff-Berechnung. Ein zweites Modul `stammdaten.rs` kapselt CRUD für Schuljahr/Klasse/Schüler. Tauri-Commands binden Preview + Apply an das Frontend. Das Frontend bekommt eine neue Admin-only-Route `/admin/stammdaten` mit Wizard-Flow (Datei wählen → Preview → Bestätigen).

**Tech Stack:** Zusätzlich zu Plan 1: `calamine 0.26` (XLSX-Reader, pure Rust, keine Office-Abhängigkeit), `rust_xlsxwriter 0.77` (nur als dev-dep, um Test-Fixtures zu generieren).

**Spec-Bezug:** Abschnitte „ASV-BW-Stammdaten-Import" und „Migration aus Legacy-Jiraso" aus `docs/superpowers/specs/2026-04-21-jiraso-reloaded-design.md`.

**Nicht im Scope** (kommt in Folge-Plänen): Fächer-/Kategorie-/Formulierungs-Import (Plan 4 Admin-Editor), Bewertungs-UI (Plan 3), PDF-Druck (Plan 5).

---

## Datei-Struktur nach Plan 2

```
jiraso-reloaded/
├── src-tauri/
│   ├── src/
│   │   ├── import.rs              # NEW: XLSX-Parser + Heuristik + Diff
│   │   ├── stammdaten.rs          # NEW: schuljahr/klasse/schueler CRUD
│   │   ├── commands.rs            # MOD: +list_schuljahre, +schuljahr_anlegen, +aktivieren, +import_preview, +import_apply
│   │   ├── lib.rs                 # MOD: mod imports + invoke_handler
│   │   └── migrations/
│   │       └── 002_schuljahr_aktiv.sql  # NEW: partial unique index für aktiv=1
│   ├── Cargo.toml                 # MOD: +calamine, +rust_xlsxwriter (dev)
│   └── tests/
│       └── fixtures/
│           └── .gitkeep           # Fixtures werden per Test generiert, nicht eingecheckt
├── src/
│   ├── lib/
│   │   └── api.ts                 # MOD: +typed wrappers für import/stammdaten
│   └── routes/
│       ├── admin/
│       │   └── stammdaten/
│       │       └── +page.svelte   # NEW: Import-Wizard (Datei → Preview → Apply)
│       └── dashboard/+page.svelte # MOD: Link für Admin zu /admin/stammdaten
├── import-vorlagen/
│   ├── README.md                  # NEW: Anleitung zur ASV-BW-Exportvorlage
│   └── asv-export-template.md     # NEW: Platzhalter (echte .exf wird vom User geliefert)
└── docs/superpowers/plans/
    └── 2026-04-23-plan-2-stammdaten-import.md  # dieses Dokument
```

**Verantwortlichkeiten:**
- `import.rs` kennt ausschließlich XLSX-Bytes → strukturierte Records. Keine DB-Zugriffe, keine Tauri-Types. Erleichtert Unit-Tests.
- `stammdaten.rs` kennt ausschließlich DB-Operationen auf schuljahr/klasse/schueler-Tabellen. Keine XLSX-Kenntnisse.
- `commands.rs` orchestriert: liest XLSX-Bytes via `import::parse_and_match`, ruft `stammdaten::apply_diff` auf.

---

## Task 1: Dependencies hinzufügen

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [x] **Step 1: `[dependencies]`- und `[dev-dependencies]`-Blöcke erweitern**

In `src-tauri/Cargo.toml` den bestehenden `[dependencies]`-Block erweitern (nur die neue Zeile anhängen, die anderen unberührt lassen):

```toml
calamine = "0.26"
```

Und den `[dev-dependencies]`-Block erweitern:

```toml
rust_xlsxwriter = "0.77"
```

- [x] **Step 2: `cargo check` im Backend**

```bash
cd src-tauri && cargo check --no-default-features --lib
```

Expected: Dependencies werden aufgelöst, keine Fehler.

- [x] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore(deps): add calamine + rust_xlsxwriter dev-dep for Stammdaten-Import

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Migration 002 — Schuljahr-aktiv Constraint

**Files:**
- Create: `src-tauri/src/migrations/002_schuljahr_aktiv.sql`
- Modify: `src-tauri/src/db.rs`

Ziel: Genau ein Schuljahr mit `aktiv=1`. SQLite-Partial-Unique-Index.

- [x] **Step 1: SQL-Datei anlegen**

```sql
-- src-tauri/src/migrations/002_schuljahr_aktiv.sql
-- Nur ein Schuljahr darf aktiv=1 haben; aktiv=0 beliebig oft.
CREATE UNIQUE INDEX idx_schuljahr_nur_eins_aktiv
    ON schuljahr(aktiv) WHERE aktiv = 1;
```

- [x] **Step 2: Migration im Runner registrieren**

Ersetze in `src-tauri/src/db.rs` den Migrations-Block:

```rust
// src-tauri/src/db.rs
static MIGRATIONS_001: &str = include_str!("migrations/001_initial.sql");
static MIGRATIONS_002: &str = include_str!("migrations/002_schuljahr_aktiv.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(MIGRATIONS_001),
        M::up(MIGRATIONS_002),
    ])
}
```

- [x] **Step 3: Test: Migration läuft idempotent und Constraint wirkt**

Erweitere den bestehenden `tests`-Block in `src-tauri/src/db.rs`:

```rust
#[test]
fn only_one_active_schuljahr_allowed() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    let conn = open(&path).unwrap();

    conn.execute(
        "INSERT INTO schuljahr(bezeichnung, aktiv) VALUES ('2025/26', 1)",
        [],
    ).unwrap();

    // Zweites aktives darf NICHT gehen
    let err = conn.execute(
        "INSERT INTO schuljahr(bezeichnung, aktiv) VALUES ('2026/27', 1)",
        [],
    );
    assert!(err.is_err(), "Zwei aktive Schuljahre dürfen nicht koexistieren");

    // Inaktives daneben ist ok
    conn.execute(
        "INSERT INTO schuljahr(bezeichnung, aktiv) VALUES ('2026/27', 0)",
        [],
    ).unwrap();
}

#[test]
fn reopening_db_does_not_reapply_migrations() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    { let _conn = open(&path).unwrap(); }
    let conn = open(&path).unwrap();
    // Falls Migrations versehentlich erneut liefen, würde CREATE INDEX ohne IF NOT EXISTS scheitern.
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_schuljahr_nur_eins_aktiv'",
        [], |r| r.get(0),
    ).unwrap();
    assert_eq!(count, 1);
}
```

- [x] **Step 4: Tests laufen lassen**

```bash
cd src-tauri && cargo test --no-default-features --lib db::
```

Expected: neu geschriebene Tests grün, bestehende Tests weiterhin grün.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/migrations/002_schuljahr_aktiv.sql src-tauri/src/db.rs
git commit -m "feat(db): migration 002 enforces single active Schuljahr

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: `stammdaten`-Modul — Schuljahr-CRUD (TDD)

**Files:**
- Create: `src-tauri/src/stammdaten.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Modul-Grundgerüst + Schuljahr-Typ**

Lege `src-tauri/src/stammdaten.rs` an:

```rust
// src-tauri/src/stammdaten.rs
use crate::error::AppResult;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Schuljahr {
    pub id: i64,
    pub bezeichnung: String,
    pub aktiv: bool,
}

pub fn list_schuljahre(conn: &Connection) -> AppResult<Vec<Schuljahr>> {
    let mut stmt = conn.prepare(
        "SELECT id, bezeichnung, aktiv FROM schuljahr ORDER BY bezeichnung DESC",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(Schuljahr {
            id: r.get(0)?,
            bezeichnung: r.get(1)?,
            aktiv: r.get::<_, i64>(2)? == 1,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn aktives_schuljahr(conn: &Connection) -> AppResult<Option<Schuljahr>> {
    let mut stmt = conn.prepare(
        "SELECT id, bezeichnung, aktiv FROM schuljahr WHERE aktiv=1 LIMIT 1",
    )?;
    let mut rows = stmt.query_map([], |r| {
        Ok(Schuljahr {
            id: r.get(0)?,
            bezeichnung: r.get(1)?,
            aktiv: r.get::<_, i64>(2)? == 1,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn anlegen(conn: &Connection, bezeichnung: &str) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO schuljahr(bezeichnung, aktiv) VALUES (?1, 0)",
        params![bezeichnung],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Setzt ein Schuljahr aktiv; alle anderen werden inaktiv.
pub fn aktivieren(conn: &mut Connection, id: i64) -> AppResult<()> {
    let tx = conn.transaction()?;
    tx.execute("UPDATE schuljahr SET aktiv = 0", [])?;
    let changed = tx.execute(
        "UPDATE schuljahr SET aktiv = 1 WHERE id = ?1",
        params![id],
    )?;
    if changed == 0 {
        return Err(crate::error::AppError::Config(format!(
            "Schuljahr mit id={id} existiert nicht",
        )));
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    fn fresh_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("t.db")).unwrap();
        (dir, conn)
    }

    #[test]
    fn anlegen_und_auflisten() {
        let (_d, conn) = fresh_conn();
        anlegen(&conn, "2025/26").unwrap();
        anlegen(&conn, "2026/27").unwrap();
        let list = list_schuljahre(&conn).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].bezeichnung, "2026/27"); // DESC
        assert!(!list[0].aktiv);
    }

    #[test]
    fn aktivieren_setzt_alle_anderen_auf_inaktiv() {
        let (_d, mut conn) = fresh_conn();
        let a = anlegen(&conn, "2025/26").unwrap();
        let b = anlegen(&conn, "2026/27").unwrap();
        aktivieren(&mut conn, a).unwrap();
        assert_eq!(aktives_schuljahr(&conn).unwrap().unwrap().id, a);
        aktivieren(&mut conn, b).unwrap();
        assert_eq!(aktives_schuljahr(&conn).unwrap().unwrap().id, b);
        let all = list_schuljahre(&conn).unwrap();
        let aktiv_count = all.iter().filter(|s| s.aktiv).count();
        assert_eq!(aktiv_count, 1);
    }

    #[test]
    fn aktivieren_unbekannter_id_fehlschlaegt() {
        let (_d, mut conn) = fresh_conn();
        let err = aktivieren(&mut conn, 999);
        assert!(err.is_err());
    }
}
```

- [x] **Step 2: Modul in `lib.rs` registrieren**

Füge in `src-tauri/src/lib.rs` nach `pub mod backup;` eine Zeile hinzu:

```rust
pub mod stammdaten;
```

- [x] **Step 3: Tests**

```bash
cd src-tauri && cargo test --no-default-features --lib stammdaten::
```

Expected: 3 neue Tests grün.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/stammdaten.rs src-tauri/src/lib.rs
git commit -m "feat(stammdaten): Schuljahr CRUD (list, anlegen, aktivieren)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: `stammdaten`-Modul — Klasse + Schüler upsert (TDD)

**Files:**
- Modify: `src-tauri/src/stammdaten.rs`

- [x] **Step 1: Typen + Upsert-Funktionen**

Ergänze am Ende von `src-tauri/src/stammdaten.rs` (vor dem `#[cfg(test)] mod tests`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchuelerInput {
    pub asv_uuid: Option<String>,
    pub klasse: String,
    pub vorname: String,
    pub nachname: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportSummary {
    pub neue_klassen: usize,
    pub neue_schueler: usize,
    pub geaenderte_schueler: usize,
    pub unveraenderte_schueler: usize,
}

fn klasse_id_or_insert(
    conn: &Connection,
    schuljahr_id: i64,
    name: &str,
) -> AppResult<(i64, bool)> {
    if let Some(id) = conn.query_row(
        "SELECT id FROM klasse WHERE schuljahr_id=?1 AND name=?2",
        params![schuljahr_id, name],
        |r| r.get::<_, i64>(0),
    ).optional()? {
        return Ok((id, false));
    }
    conn.execute(
        "INSERT INTO klasse(schuljahr_id, name) VALUES (?1, ?2)",
        params![schuljahr_id, name],
    )?;
    Ok((conn.last_insert_rowid(), true))
}

/// Schreibt den Input-Batch ins Schuljahr. Match-Reihenfolge:
///   1. asv_uuid (falls vorhanden und bereits in DB): update
///   2. Fallback (vorname, nachname, klasse): update
///   3. sonst insert
pub fn upsert_schueler(
    conn: &mut Connection,
    schuljahr_id: i64,
    input: &[SchuelerInput],
) -> AppResult<ImportSummary> {
    let tx = conn.transaction()?;
    let mut summary = ImportSummary::default();

    for s in input {
        let (klasse_id, klasse_neu) =
            klasse_id_or_insert_tx(&tx, schuljahr_id, &s.klasse)?;
        if klasse_neu { summary.neue_klassen += 1; }

        let existing: Option<(i64, String, String, i64)> = if let Some(uuid) = &s.asv_uuid {
            tx.query_row(
                "SELECT id, vorname, nachname, klasse_id
                 FROM schueler WHERE asv_uuid = ?1",
                params![uuid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ).optional()?
        } else {
            tx.query_row(
                "SELECT id, vorname, nachname, klasse_id
                 FROM schueler
                 WHERE asv_uuid IS NULL
                   AND vorname = ?1 AND nachname = ?2
                   AND klasse_id IN (SELECT id FROM klasse WHERE schuljahr_id = ?3)",
                params![s.vorname, s.nachname, schuljahr_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ).optional()?
        };

        match existing {
            Some((id, v, n, k)) if v == s.vorname && n == s.nachname && k == klasse_id => {
                summary.unveraenderte_schueler += 1;
            }
            Some((id, _, _, _)) => {
                tx.execute(
                    "UPDATE schueler SET vorname=?1, nachname=?2, klasse_id=?3 WHERE id=?4",
                    params![s.vorname, s.nachname, klasse_id, id],
                )?;
                summary.geaenderte_schueler += 1;
            }
            None => {
                tx.execute(
                    "INSERT INTO schueler(asv_uuid, klasse_id, vorname, nachname)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![s.asv_uuid, klasse_id, s.vorname, s.nachname],
                )?;
                summary.neue_schueler += 1;
            }
        }
    }

    tx.commit()?;
    Ok(summary)
}

fn klasse_id_or_insert_tx(
    tx: &rusqlite::Transaction,
    schuljahr_id: i64,
    name: &str,
) -> AppResult<(i64, bool)> {
    if let Some(id) = tx.query_row(
        "SELECT id FROM klasse WHERE schuljahr_id=?1 AND name=?2",
        params![schuljahr_id, name],
        |r| r.get::<_, i64>(0),
    ).optional()? {
        return Ok((id, false));
    }
    tx.execute(
        "INSERT INTO klasse(schuljahr_id, name) VALUES (?1, ?2)",
        params![schuljahr_id, name],
    )?;
    Ok((tx.last_insert_rowid(), true))
}
```

Ergänze oben im File den use-Statement:

```rust
use rusqlite::OptionalExtension;
```

- [x] **Step 2: Tests am Ende von `tests`-Modul ergänzen**

```rust
    fn sample_input(uuid: Option<&str>, klasse: &str, vn: &str, nn: &str) -> SchuelerInput {
        SchuelerInput {
            asv_uuid: uuid.map(String::from),
            klasse: klasse.into(),
            vorname: vn.into(),
            nachname: nn.into(),
        }
    }

    #[test]
    fn upsert_legt_klassen_und_schueler_neu_an() {
        let (_d, mut conn) = fresh_conn();
        let sj = anlegen(&conn, "2025/26").unwrap();
        let sum = upsert_schueler(&mut conn, sj, &[
            sample_input(Some("uuid-1"), "5a", "Anna", "Apfel"),
            sample_input(Some("uuid-2"), "5a", "Bert", "Birne"),
            sample_input(Some("uuid-3"), "5b", "Cora", "Citro"),
        ]).unwrap();
        assert_eq!(sum.neue_klassen, 2);
        assert_eq!(sum.neue_schueler, 3);
        assert_eq!(sum.geaenderte_schueler, 0);
        assert_eq!(sum.unveraenderte_schueler, 0);
    }

    #[test]
    fn re_import_mit_unveraenderten_daten_zaehlt_als_unveraendert() {
        let (_d, mut conn) = fresh_conn();
        let sj = anlegen(&conn, "2025/26").unwrap();
        let batch = vec![sample_input(Some("uuid-1"), "5a", "Anna", "Apfel")];
        upsert_schueler(&mut conn, sj, &batch).unwrap();
        let sum = upsert_schueler(&mut conn, sj, &batch).unwrap();
        assert_eq!(sum.neue_schueler, 0);
        assert_eq!(sum.unveraenderte_schueler, 1);
    }

    #[test]
    fn re_import_mit_namensaenderung_updated_ueber_uuid() {
        let (_d, mut conn) = fresh_conn();
        let sj = anlegen(&conn, "2025/26").unwrap();
        upsert_schueler(&mut conn, sj, &[
            sample_input(Some("uuid-1"), "5a", "Anna", "Apfel"),
        ]).unwrap();
        let sum = upsert_schueler(&mut conn, sj, &[
            sample_input(Some("uuid-1"), "5b", "Anna", "Birnbaum"),
        ]).unwrap();
        assert_eq!(sum.neue_schueler, 0);
        assert_eq!(sum.geaenderte_schueler, 1);
        // Verify DB-State
        let (nn, kname): (String, String) = conn.query_row(
            "SELECT s.nachname, k.name FROM schueler s JOIN klasse k ON k.id=s.klasse_id WHERE s.asv_uuid='uuid-1'",
            [], |r| Ok((r.get(0)?, r.get(1)?)),
        ).unwrap();
        assert_eq!(nn, "Birnbaum");
        assert_eq!(kname, "5b");
    }

    #[test]
    fn fallback_match_ohne_uuid_via_name_und_klasse() {
        let (_d, mut conn) = fresh_conn();
        let sj = anlegen(&conn, "2025/26").unwrap();
        upsert_schueler(&mut conn, sj, &[
            sample_input(None, "5a", "Anna", "Apfel"),
        ]).unwrap();
        let sum = upsert_schueler(&mut conn, sj, &[
            sample_input(None, "5a", "Anna", "Apfel"),
        ]).unwrap();
        assert_eq!(sum.unveraenderte_schueler, 1);
        assert_eq!(sum.neue_schueler, 0);
    }

    #[test]
    fn upsert_ist_atomar_bei_fehler() {
        let (_d, mut conn) = fresh_conn();
        let sj = anlegen(&conn, "2025/26").unwrap();
        // Zweite Zeile hat leeren Namen — manuell injizierter Fehler: wir simulieren via SQL-Fehler
        // (leere Strings sind erlaubt; stattdessen: doppelte UUID in zwei Rows triggert UNIQUE-Fehler)
        let err = upsert_schueler(&mut conn, sj, &[
            sample_input(Some("dup-uuid"), "5a", "Anna", "Apfel"),
            sample_input(Some("dup-uuid"), "5b", "Bert", "Birne"),
        ]);
        assert!(err.is_err(), "Doppelte UUID muss Fehler werfen");
        // Rollback: Anna darf NICHT in der DB stehen
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schueler", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count, 0, "Atomarer Rollback schlug fehl");
    }
```

Hinweis: Der letzte Test nutzt aus, dass `upsert_schueler` beim zweiten Insert-Versuch (gleiche UUID) scheitert, weil die erste Zeile schon geschrieben wurde. Die Transaktion rollt zurück.

- [x] **Step 3: Tests**

```bash
cd src-tauri && cargo test --no-default-features --lib stammdaten::
```

Expected: 5 neue Tests (+ 3 bestehende) grün, 8/8.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/stammdaten.rs
git commit -m "feat(stammdaten): upsert_schueler mit UUID+Fallback-Matching

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Import-Modul — Spalten-Heuristik (TDD)

**Files:**
- Create: `src-tauri/src/import.rs`
- Modify: `src-tauri/src/lib.rs`

- [x] **Step 1: Heuristik-Funktion + Typen**

Lege `src-tauri/src/import.rs` an:

```rust
// src-tauri/src/import.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldKind {
    Uuid,
    Klasse,
    Nachname,
    Vorname,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColumnMapping {
    /// Column-Index (0-based) pro erkanntem Feld.
    pub uuid: Option<usize>,
    pub klasse: usize,
    pub nachname: usize,
    pub vorname: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DetectResult {
    /// Alle Pflicht-Felder (Klasse, Nachname, Vorname) eindeutig erkannt.
    /// UUID ist optional.
    Ok(ColumnMapping),
    /// Mehrdeutigkeit oder Fehlfeld — Frontend muss manuell zuordnen.
    Ambiguous {
        headers: Vec<String>,
        suggestions: std::collections::HashMap<FieldKind, Vec<usize>>,
    },
}

/// Normalisiert einen Header für den Vergleich: lowercase, ohne Umlaute,
/// ohne Leerzeichen / Sonderzeichen.
fn normalize(h: &str) -> String {
    h.chars()
        .flat_map(|c| match c {
            'ä' | 'Ä' => "ae".chars().collect::<Vec<_>>(),
            'ö' | 'Ö' => "oe".chars().collect::<Vec<_>>(),
            'ü' | 'Ü' => "ue".chars().collect::<Vec<_>>(),
            'ß' => "ss".chars().collect::<Vec<_>>(),
            c if c.is_alphanumeric() => c.to_lowercase().collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}

fn keywords(kind: FieldKind) -> &'static [&'static str] {
    match kind {
        FieldKind::Uuid => &["uuid", "asvuuid", "id", "schuelerid"],
        FieldKind::Klasse => &["klasse", "klassenbezeichnung", "klassestufe"],
        FieldKind::Nachname => &["nachname", "familienname", "name"],
        FieldKind::Vorname => &["vorname", "rufname"],
    }
}

pub fn detect_columns(headers: &[String]) -> DetectResult {
    use std::collections::HashMap;
    let normalized: Vec<String> = headers.iter().map(|h| normalize(h)).collect();
    let mut suggestions: HashMap<FieldKind, Vec<usize>> = HashMap::new();

    for kind in [FieldKind::Uuid, FieldKind::Klasse, FieldKind::Nachname, FieldKind::Vorname] {
        let mut matches: Vec<usize> = Vec::new();
        for (idx, n) in normalized.iter().enumerate() {
            if keywords(kind).iter().any(|kw| n == kw) {
                matches.push(idx);
            }
        }
        // Wenn kein exakter Treffer: erlaube „contains" als schwächere Stufe
        if matches.is_empty() {
            for (idx, n) in normalized.iter().enumerate() {
                if keywords(kind).iter().any(|kw| n.contains(kw)) {
                    matches.push(idx);
                }
            }
        }
        suggestions.insert(kind, matches);
    }

    // Pflicht: Klasse, Nachname, Vorname jeweils genau 1 Treffer
    let klasse = single_or_none(&suggestions[&FieldKind::Klasse]);
    let nachname = single_or_none(&suggestions[&FieldKind::Nachname]);
    let vorname = single_or_none(&suggestions[&FieldKind::Vorname]);
    let uuid = single_or_none(&suggestions[&FieldKind::Uuid]);

    // Wenn nachname und vorname auf dieselbe Spalte zeigen, ist das mehrdeutig
    let spalten_konflikt = matches!((nachname, vorname), (Some(a), Some(b)) if a == b);

    if let (Some(k), Some(n), Some(v)) = (klasse, nachname, vorname) {
        if !spalten_konflikt {
            return DetectResult::Ok(ColumnMapping {
                uuid, klasse: k, nachname: n, vorname: v,
            });
        }
    }

    DetectResult::Ambiguous {
        headers: headers.to_vec(),
        suggestions,
    }
}

fn single_or_none(matches: &[usize]) -> Option<usize> {
    if matches.len() == 1 { Some(matches[0]) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(cols: &[&str]) -> Vec<String> {
        cols.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exact_header_match() {
        let r = detect_columns(&hdr(&["ASV-UUID", "Klasse", "Nachname", "Vorname"]));
        assert_eq!(r, DetectResult::Ok(ColumnMapping {
            uuid: Some(0), klasse: 1, nachname: 2, vorname: 3,
        }));
    }

    #[test]
    fn uuid_optional() {
        let r = detect_columns(&hdr(&["Klasse", "Nachname", "Vorname"]));
        assert_eq!(r, DetectResult::Ok(ColumnMapping {
            uuid: None, klasse: 0, nachname: 1, vorname: 2,
        }));
    }

    #[test]
    fn umlaute_werden_normalisiert() {
        let r = detect_columns(&hdr(&["Klässe", "Familienname", "Rufname"]));
        assert!(matches!(r, DetectResult::Ok(_)));
    }

    #[test]
    fn ambiguous_wenn_name_doppelt() {
        let r = detect_columns(&hdr(&["Klasse", "Name", "Name"]));
        match r {
            DetectResult::Ambiguous { suggestions, .. } => {
                assert!(suggestions[&FieldKind::Nachname].len() > 1);
            }
            _ => panic!("expected ambiguous"),
        }
    }

    #[test]
    fn ambiguous_wenn_vorname_fehlt() {
        let r = detect_columns(&hdr(&["Klasse", "Nachname"]));
        assert!(matches!(r, DetectResult::Ambiguous { .. }));
    }
}
```

- [x] **Step 2: Modul registrieren**

In `src-tauri/src/lib.rs` nach `pub mod stammdaten;` einfügen:

```rust
pub mod import;
```

- [x] **Step 3: Tests**

```bash
cd src-tauri && cargo test --no-default-features --lib import::
```

Expected: 5 Tests grün.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/import.rs src-tauri/src/lib.rs
git commit -m "feat(import): column heuristic detector (pure, TDD)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Import-Modul — XLSX-Parser (TDD)

**Files:**
- Modify: `src-tauri/src/import.rs`

- [x] **Step 1: Parser-Funktion, liest XLSX-Bytes → Vec<SchuelerInput>**

Ergänze am Ende von `src-tauri/src/import.rs` (vor `#[cfg(test)]`):

```rust
use crate::error::{AppError, AppResult};
use crate::stammdaten::SchuelerInput;
use calamine::{Data, Reader, Xlsx};
use std::io::Cursor;

/// Parst XLSX-Bytes und extrahiert die erste Tabelle.
/// Erwartet eine Header-Zeile oben; weitere Zeilen sind Datensätze.
/// Leere Zeilen werden übersprungen.
pub fn parse_xlsx(bytes: &[u8]) -> AppResult<ParsedSheet> {
    let mut workbook: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes))
        .map_err(|e| AppError::Config(format!("XLSX ist ungültig: {e}")))?;
    let sheet_name = workbook.sheet_names().first().cloned()
        .ok_or_else(|| AppError::Config("XLSX enthält keine Tabelle".into()))?;
    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| AppError::Config(format!("Tabelle '{sheet_name}' nicht lesbar: {e}")))?;
    let mut rows = range.rows();
    let header_row = rows.next()
        .ok_or_else(|| AppError::Config("XLSX ist leer".into()))?;
    let headers: Vec<String> = header_row.iter().map(cell_to_string).collect();
    let body: Vec<Vec<String>> = rows
        .map(|r| r.iter().map(cell_to_string).collect())
        .filter(|r: &Vec<String>| r.iter().any(|c| !c.trim().is_empty()))
        .collect();
    Ok(ParsedSheet { headers, rows: body })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSheet {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 { format!("{}", *f as i64) } else { format!("{f}") }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR:{e:?}"),
    }
}

/// Wendet ein ColumnMapping auf rohe Zeilen an → typisierte Input-Records.
/// Zeilen, bei denen Pflichtfelder (klasse, vorname, nachname) leer sind, werden verworfen.
pub fn build_inputs(sheet: &ParsedSheet, mapping: &ColumnMapping) -> Vec<SchuelerInput> {
    sheet.rows.iter().filter_map(|row| {
        let get = |idx: usize| row.get(idx).map(|s| s.trim().to_string()).unwrap_or_default();
        let klasse = get(mapping.klasse);
        let vorname = get(mapping.vorname);
        let nachname = get(mapping.nachname);
        if klasse.is_empty() || vorname.is_empty() || nachname.is_empty() {
            return None;
        }
        let asv_uuid = mapping.uuid.and_then(|i| {
            let v = get(i);
            if v.is_empty() { None } else { Some(v) }
        });
        Some(SchuelerInput { asv_uuid, klasse, vorname, nachname })
    }).collect()
}
```

- [x] **Step 2: Test-Helper, der eine XLSX im Speicher erzeugt**

Ergänze im `tests`-Modul von `import.rs`:

```rust
    use rust_xlsxwriter::Workbook;

    fn make_xlsx(rows: &[&[&str]]) -> Vec<u8> {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        for (r_idx, row) in rows.iter().enumerate() {
            for (c_idx, val) in row.iter().enumerate() {
                ws.write_string(r_idx as u32, c_idx as u16, *val).unwrap();
            }
        }
        wb.save_to_buffer().unwrap()
    }

    #[test]
    fn parse_xlsx_liest_header_und_zeilen() {
        let bytes = make_xlsx(&[
            &["ASV-UUID", "Klasse", "Nachname", "Vorname"],
            &["uuid-1", "5a", "Apfel", "Anna"],
            &["uuid-2", "5a", "Birne", "Bert"],
            &["", "", "", ""],   // leere Zeile wird verworfen
            &["uuid-3", "5b", "Citro", "Cora"],
        ]);
        let sheet = parse_xlsx(&bytes).unwrap();
        assert_eq!(sheet.headers, vec!["ASV-UUID", "Klasse", "Nachname", "Vorname"]);
        assert_eq!(sheet.rows.len(), 3);
    }

    #[test]
    fn build_inputs_verwirft_zeilen_ohne_pflichtfelder() {
        let sheet = ParsedSheet {
            headers: vec!["Klasse".into(), "Nachname".into(), "Vorname".into()],
            rows: vec![
                vec!["5a".into(), "Apfel".into(), "Anna".into()],
                vec!["5a".into(), "".into(), "Bert".into()],   // Nachname leer → skip
            ],
        };
        let mapping = ColumnMapping {
            uuid: None, klasse: 0, nachname: 1, vorname: 2,
        };
        let input = build_inputs(&sheet, &mapping);
        assert_eq!(input.len(), 1);
        assert_eq!(input[0].vorname, "Anna");
    }

    #[test]
    fn build_inputs_behandelt_leere_uuid_als_none() {
        let sheet = ParsedSheet {
            headers: vec!["UUID".into(), "Klasse".into(), "Nachname".into(), "Vorname".into()],
            rows: vec![
                vec!["uuid-1".into(), "5a".into(), "Apfel".into(), "Anna".into()],
                vec!["".into(), "5a".into(), "Birne".into(), "Bert".into()],
            ],
        };
        let mapping = ColumnMapping {
            uuid: Some(0), klasse: 1, nachname: 2, vorname: 3,
        };
        let input = build_inputs(&sheet, &mapping);
        assert_eq!(input[0].asv_uuid, Some("uuid-1".into()));
        assert_eq!(input[1].asv_uuid, None);
    }
```

- [x] **Step 3: Tests**

```bash
cd src-tauri && cargo test --no-default-features --lib import::
```

Expected: 8 Tests grün.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/import.rs
git commit -m "feat(import): calamine XLSX parser + build_inputs (TDD)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Tauri-Commands für Import + Stammdaten

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs` (invoke_handler)

- [x] **Step 1: Commands hinzufügen**

Ergänze am Ende von `src-tauri/src/commands.rs`:

```rust
use crate::import::{self, ColumnMapping, DetectResult, ParsedSheet};
use crate::stammdaten::{self, ImportSummary, Schuljahr};
use crate::error::{AppError, AppResult};

fn require_admin(state: &tauri::State<AppState>) -> AppResult<()> {
    match *state.rolle.lock().unwrap() {
        Some(crate::auth::Rolle::Admin) => Ok(()),
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
pub fn import_xlsx_preview(
    bytes: Vec<u8>,
    state: tauri::State<AppState>,
) -> AppResult<ImportPreview> {
    require_admin(&state)?;
    let sheet = import::parse_xlsx(&bytes)?;
    let detection = import::detect_columns(&sheet.headers);
    Ok(ImportPreview { sheet, detection })
}

#[tauri::command]
pub fn import_xlsx_apply(
    schuljahr_id: i64,
    bytes: Vec<u8>,
    mapping: ColumnMapping,
    state: tauri::State<AppState>,
) -> AppResult<ImportSummary> {
    require_admin(&state)?;
    let sheet = import::parse_xlsx(&bytes)?;
    let records = import::build_inputs(&sheet, &mapping);
    let mut conn = open_db(&state)?;
    stammdaten::upsert_schueler(&mut conn, schuljahr_id, &records)
}
```

- [x] **Step 2: invoke_handler erweitern**

Ersetze in `src-tauri/src/lib.rs` den `invoke_handler!`-Aufruf durch die erweiterte Variante (alle bisherigen Commands bleiben, neue kommen hinzu):

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
            commands::list_schuljahre,
            commands::schuljahr_anlegen,
            commands::schuljahr_aktivieren,
            commands::import_xlsx_preview,
            commands::import_xlsx_apply,
        ])
```

- [x] **Step 3: Backend-Compile prüfen**

```bash
cd src-tauri && cargo check --no-default-features --lib
cd src-tauri && cargo check --lib
```

Expected: beide grün. Die zweite Variante prüft mit `desktop`-Feature (braucht webkit2gtk) — falls das lokal scheitert, nur den No-Default-Lauf als Pflicht werten und im CI auf Windows vertrauen.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(commands): stammdaten + import tauri commands (admin-only)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Frontend API-Wrapper

**Files:**
- Modify: `src/lib/api.ts`

- [x] **Step 1: Typen + Wrapper ergänzen**

Hänge ans Ende von `src/lib/api.ts` folgendes an:

```ts
// --- Stammdaten / Import ---

export type Schuljahr = {
  id: number;
  bezeichnung: string;
  aktiv: boolean;
};

export type ColumnMapping = {
  uuid: number | null;
  klasse: number;
  nachname: number;
  vorname: number;
};

export type FieldKind = "Uuid" | "Klasse" | "Nachname" | "Vorname";

export type DetectResult =
  | { Ok: ColumnMapping }
  | {
      Ambiguous: {
        headers: string[];
        suggestions: Record<FieldKind, number[]>;
      };
    };

export type ParsedSheet = {
  headers: string[];
  rows: string[][];
};

export type ImportPreview = {
  sheet: ParsedSheet;
  detection: DetectResult;
};

export type ImportSummary = {
  neue_klassen: number;
  neue_schueler: number;
  geaenderte_schueler: number;
  unveraenderte_schueler: number;
};

export const stammdaten = {
  list: () => invoke<Schuljahr[]>("list_schuljahre"),
  anlegen: (bezeichnung: string, aktivieren: boolean) =>
    invoke<number>("schuljahr_anlegen", { bezeichnung, aktivieren }),
  aktivieren: (id: number) => invoke<void>("schuljahr_aktivieren", { id }),
};

export const importXlsx = {
  preview: (bytes: number[]) =>
    invoke<ImportPreview>("import_xlsx_preview", { bytes }),
  apply: (schuljahrId: number, bytes: number[], mapping: ColumnMapping) =>
    invoke<ImportSummary>("import_xlsx_apply", {
      schuljahrId,
      bytes,
      mapping,
    }),
};
```

Hinweis: Tauri verwendet `camelCase` in den Invoke-Argumenten, auch wenn die Rust-Seite `snake_case` hat — das ist das Standardverhalten von `tauri::command`.

- [x] **Step 2: Typecheck**

```bash
npm run check
```

Expected: 0 errors.

- [x] **Step 3: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(frontend): typed API wrappers for stammdaten + import

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Frontend-Route `/admin/stammdaten`

**Files:**
- Create: `src/routes/admin/stammdaten/+page.svelte`

- [x] **Step 1: Page-Komponente mit Wizard-Flow**

Lege `src/routes/admin/stammdaten/+page.svelte` an:

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { session } from "$lib/session.svelte";
  import {
    stammdaten,
    importXlsx,
    type Schuljahr,
    type ColumnMapping,
    type DetectResult,
    type ImportPreview,
    type ImportSummary,
  } from "$lib/api";

  let schuljahre = $state<Schuljahr[]>([]);
  let ausgewaehltesSchuljahr = $state<number | null>(null);
  let neueBezeichnung = $state("");
  let schritt = $state<"auswahl" | "datei" | "preview" | "fertig">("auswahl");
  let preview = $state<ImportPreview | null>(null);
  let mapping = $state<ColumnMapping | null>(null);
  let bytesBuffer = $state<number[] | null>(null);
  let summary = $state<ImportSummary | null>(null);
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (session.rolle !== "Admin") {
      goto("/dashboard");
      return;
    }
    await refresh();
  });

  async function refresh() {
    schuljahre = await stammdaten.list();
  }

  async function neuesSchuljahr() {
    fehler = null;
    if (!/^\d{4}\/\d{2}$/.test(neueBezeichnung.trim())) {
      fehler = "Format: YYYY/YY (z.B. 2025/26)";
      return;
    }
    try {
      const id = await stammdaten.anlegen(neueBezeichnung.trim(), true);
      neueBezeichnung = "";
      await refresh();
      ausgewaehltesSchuljahr = id;
      schritt = "datei";
    } catch (e) {
      fehler = String(e);
    }
  }

  function weiterZuDatei() {
    if (ausgewaehltesSchuljahr === null) {
      fehler = "Bitte zuerst ein Schuljahr wählen oder anlegen.";
      return;
    }
    fehler = null;
    schritt = "datei";
  }

  async function dateiGewaehlt(e: Event) {
    const input = e.target as HTMLInputElement;
    const f = input.files?.[0];
    if (!f) return;
    fehler = null;
    try {
      const buf = new Uint8Array(await f.arrayBuffer());
      bytesBuffer = Array.from(buf);
      preview = await importXlsx.preview(bytesBuffer);
      if ("Ok" in preview.detection) {
        mapping = preview.detection.Ok;
      } else {
        // Initialer Mapping-Vorschlag aus den Suggestions (nimm jeweils ersten Kandidaten)
        const s = preview.detection.Ambiguous.suggestions;
        mapping = {
          uuid: s.Uuid?.[0] ?? null,
          klasse: s.Klasse?.[0] ?? 0,
          nachname: s.Nachname?.[0] ?? 0,
          vorname: s.Vorname?.[0] ?? 0,
        };
      }
      schritt = "preview";
    } catch (err) {
      fehler = String(err);
    }
  }

  async function anwenden() {
    if (!bytesBuffer || !mapping || ausgewaehltesSchuljahr === null) return;
    fehler = null;
    try {
      summary = await importXlsx.apply(
        ausgewaehltesSchuljahr,
        bytesBuffer,
        mapping,
      );
      schritt = "fertig";
      await refresh();
    } catch (err) {
      fehler = String(err);
    }
  }

  function zurueck() {
    schritt = "auswahl";
    preview = null;
    mapping = null;
    bytesBuffer = null;
    summary = null;
    fehler = null;
  }
</script>

<main class="container">
  <header>
    <h1>Stammdaten-Import</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}
    <p class="error">Fehler: {fehler}</p>
  {/if}

  {#if schritt === "auswahl"}
    <section>
      <h2>1. Schuljahr wählen</h2>
      {#if schuljahre.length > 0}
        <ul>
          {#each schuljahre as sj}
            <li>
              <label>
                <input
                  type="radio"
                  bind:group={ausgewaehltesSchuljahr}
                  value={sj.id}
                />
                {sj.bezeichnung} {sj.aktiv ? "(aktiv)" : ""}
              </label>
            </li>
          {/each}
        </ul>
      {:else}
        <p><em>Noch kein Schuljahr angelegt.</em></p>
      {/if}

      <h3>oder neu anlegen</h3>
      <input
        placeholder="2025/26"
        bind:value={neueBezeichnung}
      />
      <button onclick={neuesSchuljahr}>Anlegen + aktivieren</button>

      <div class="actions">
        <button onclick={weiterZuDatei} disabled={ausgewaehltesSchuljahr === null}>
          Weiter
        </button>
      </div>
    </section>
  {/if}

  {#if schritt === "datei"}
    <section>
      <h2>2. XLSX-Datei aus ASV-BW wählen</h2>
      <p>Erwartete Spalten: ASV-UUID (optional), Klasse, Nachname, Vorname.</p>
      <input type="file" accept=".xlsx" onchange={dateiGewaehlt} />
      <button onclick={zurueck}>Abbrechen</button>
    </section>
  {/if}

  {#if schritt === "preview" && preview && mapping}
    <section>
      <h2>3. Vorschau &amp; Spalten-Zuordnung</h2>
      <p>
        {preview.sheet.rows.length} Datenzeile(n) erkannt,
        {preview.sheet.headers.length} Spalten.
      </p>

      <table class="mapping">
        <thead>
          <tr>
            <th>Spalte in XLSX</th>
            {#each preview.sheet.headers as h, i}
              <th>{i}: {h}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each ["Klasse", "Nachname", "Vorname", "UUID (optional)"] as label, row}
            <tr>
              <td>{label}</td>
              {#each preview.sheet.headers as _h, colIdx}
                <td>
                  <input
                    type="radio"
                    name={`map-${row}`}
                    checked={
                      (row === 0 && mapping.klasse === colIdx) ||
                      (row === 1 && mapping.nachname === colIdx) ||
                      (row === 2 && mapping.vorname === colIdx) ||
                      (row === 3 && mapping.uuid === colIdx)
                    }
                    onchange={() => {
                      if (row === 0) mapping = { ...mapping!, klasse: colIdx };
                      else if (row === 1) mapping = { ...mapping!, nachname: colIdx };
                      else if (row === 2) mapping = { ...mapping!, vorname: colIdx };
                      else mapping = { ...mapping!, uuid: colIdx };
                    }}
                  />
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>

      <h3>Erste 5 Datenzeilen</h3>
      <table class="sample">
        <thead>
          <tr>
            {#each preview.sheet.headers as h}
              <th>{h}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each preview.sheet.rows.slice(0, 5) as row}
            <tr>
              {#each row as cell}
                <td>{cell}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>

      <div class="actions">
        <button onclick={zurueck}>Abbrechen</button>
        <button onclick={anwenden}>Importieren</button>
      </div>
    </section>
  {/if}

  {#if schritt === "fertig" && summary}
    <section>
      <h2>Import abgeschlossen</h2>
      <ul>
        <li>{summary.neue_klassen} neue Klasse(n)</li>
        <li>{summary.neue_schueler} neue Schüler:innen</li>
        <li>{summary.geaenderte_schueler} aktualisiert</li>
        <li>{summary.unveraenderte_schueler} unverändert</li>
      </ul>
      <div class="actions">
        <button onclick={zurueck}>Neuer Import</button>
        <a href="/dashboard">← Dashboard</a>
      </div>
    </section>
  {/if}
</main>

<style>
  .container { max-width: 1000px; margin: 0 auto; padding: 2rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  section { margin-top: 2rem; padding: 1.5rem; border: 1px solid #ccc; border-radius: 8px; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .actions { margin-top: 1.5rem; display: flex; gap: 1rem; }
  table { border-collapse: collapse; font-size: 0.85rem; margin: 1rem 0; }
  th, td { border: 1px solid #ddd; padding: 4px 8px; text-align: left; }
  table.mapping td:first-child { font-weight: bold; }
  table.sample th { background: #f5f5f5; }
</style>
```

- [x] **Step 2: Build & Typecheck**

```bash
npm run check
npm run build
```

Expected: 0 errors, Build erfolgreich.

- [x] **Step 3: Commit**

```bash
git add src/routes/admin/stammdaten/+page.svelte
git commit -m "feat(frontend): admin stammdaten import wizard

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Dashboard-Link für Admin

**Files:**
- Modify: `src/routes/dashboard/+page.svelte`

- [x] **Step 1: Conditional-Link einfügen**

Öffne `src/routes/dashboard/+page.svelte` und füge — an einer gut sichtbaren Stelle zwischen Begrüßung und evtl. anderen Kacheln — einen Admin-Link ein (wenn `session.rolle === "Admin"`):

```svelte
{#if session.rolle === "Admin"}
  <a href="/admin/stammdaten" class="admin-tile">
    <strong>Stammdaten-Import</strong>
    <span>XLSX aus ASV-BW einspielen</span>
  </a>
{/if}
```

Und ein minimales Style dazu (oder vorhandenes Kachel-CSS wiederverwenden, falls die Dashboard-Seite bereits welches hat):

```css
.admin-tile {
  display: inline-block;
  padding: 1rem 1.5rem;
  border: 1px solid #888;
  border-radius: 8px;
  text-decoration: none;
  color: inherit;
  margin-top: 1rem;
}
.admin-tile strong { display: block; }
.admin-tile span { font-size: 0.9em; color: #666; }
```

Falls das Dashboard bereits ein Style-Konvention verwendet (Kachel-Grid o. ä.), dort einfügen statt neu erfinden.

- [x] **Step 2: Build**

```bash
npm run build
```

Expected: grün.

- [x] **Step 3: Commit**

```bash
git add src/routes/dashboard/+page.svelte
git commit -m "feat(dashboard): admin link to stammdaten import

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: Import-Vorlagen-Platzhalter + README-Update

**Files:**
- Create: `import-vorlagen/README.md`
- Create: `import-vorlagen/asv-export-template.md`
- Modify: `README.md` (Root)

- [x] **Step 1: import-vorlagen/README.md anlegen**

```markdown
# Import-Vorlagen

In diesem Ordner liegen Vorlagen, die Admin-User einmalig in externe Systeme einspielen können, damit Exporte direkt im für Jiraso-reloaded passenden Format erfolgen.

## `asv-export-template.exf`

**Status: Platzhalter** — die echte `.exf`-Datei muss vom Admin in ASV-BW erzeugt und hier abgelegt werden (siehe `asv-export-template.md`).

Ziel-Export-Spalten (in dieser Reihenfolge empfohlen, Reihenfolge ist aber egal — Jiraso erkennt die Spalten heuristisch):

| Spalte | Bedeutung |
|---|---|
| `ASV-UUID` | 36-stellige eindeutige Schüler-ID, bleibt über Schuljahre stabil |
| `Klasse` | z. B. „5a", „6c" |
| `Nachname` | Familienname |
| `Vorname` | Rufname |

Format: XLSX. Header-Zeile oben. Eine Zeile pro Schüler:in.
```

- [x] **Step 2: asv-export-template.md anlegen**

```markdown
# ASV-BW-Exportvorlage für Jiraso-reloaded

Diese Anleitung beschreibt, wie eine Exportvorlage in ASV-BW angelegt wird, die einen für Jiraso passenden XLSX-Export erzeugt.

## Schritt-für-Schritt in ASV-BW

1. Menü „Auswertungen" → „Individueller Export" öffnen.
2. Export-Typ: **XLSX** wählen.
3. Spalten zufügen (per Drag & Drop):
   - `Schüler.ID` → Kopfzeile umbenennen in `ASV-UUID`
   - `Schüler.Klasse` → Kopfzeile `Klasse`
   - `Schüler.Nachname` → Kopfzeile `Nachname`
   - `Schüler.Vorname` → Kopfzeile `Vorname`
4. Filter: aktuelles Schuljahr, nur Klassen 5 und 6.
5. Vorlage speichern als `jiraso-reloaded-v1.exf`.
6. Die `.exf`-Datei in diesen Ordner kopieren und mit Jiraso-reloaded zusammen auf das Netzlaufwerk legen.

## Verwendung

- Ein Admin importiert einmalig `jiraso-reloaded-v1.exf` in seine ASV-BW-Instanz.
- Danach genügt ein Klick, um den passenden XLSX-Export zu erzeugen.
- In Jiraso-reloaded: Admin → Stammdaten-Import → XLSX wählen.

## Hinweis

Die Spalten-Erkennung in Jiraso-reloaded ist heuristisch und funktioniert auch mit anderen Kopfzeilen (z. B. „Familienname" statt „Nachname", „Rufname" statt „Vorname"). Die genaue Bezeichnung ist also nicht kritisch, solange jede Spalte eindeutig ist.
```

- [x] **Step 3: Root-README um Plan-2-Referenz ergänzen**

Suche in `README.md` den Abschnitt, der die Features listet (im Plan-1-README angelegt), und ergänze einen Unterpunkt:

```markdown
- **Stammdaten-Import** (ab Plan 2, Admin-only): XLSX-Import aus ASV-BW mit heuristischer Spalten-Erkennung, ASV-UUID-basiertem Re-Import, Schuljahr-Verwaltung.
```

Außerdem ergänze unterhalb des Setup-Abschnitts einen neuen Abschnitt:

```markdown
## Stammdaten importieren

1. Als **Admin** einloggen.
2. Dashboard → „Stammdaten-Import".
3. Schuljahr wählen oder neu anlegen (Format `YYYY/YY`, z. B. `2025/26`).
4. XLSX aus ASV-BW wählen (siehe `import-vorlagen/asv-export-template.md`).
5. Vorschau prüfen, ggf. Spalten zuordnen, bestätigen.
```

- [x] **Step 4: Commit**

```bash
git add import-vorlagen/ README.md
git commit -m "docs: import templates and README section for Stammdaten-Import

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 12: Plan durchhaken, PR öffnen, CI abwarten

**Files:**
- Modify: `docs/superpowers/plans/2026-04-23-plan-2-stammdaten-import.md` (diese Datei, alle Checkboxen `- [x]` → `- [x]`)

- [x] **Step 1: Alle bestehenden Tests laufen lassen**

```bash
cd src-tauri && cargo test --no-default-features --lib
cd .. && npm run check && npm run build
```

Expected: alles grün.

- [x] **Step 2: Plan-Checkboxen durchhaken**

Suche/Ersetze `- [x]` → `- [x]` in diesem Plan-File für alle Steps, die erledigt sind. Commit:

```bash
git add docs/superpowers/plans/2026-04-23-plan-2-stammdaten-import.md
git commit -m "docs(plan): mark Plan 2 tasks complete

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

- [x] **Step 3: Push + PR öffnen**

```bash
git push -u origin feat/stammdaten-import
gh pr create --title "Plan 2: Stammdaten-Import (XLSX aus ASV-BW)" --body "$(cat <<'EOF'
## Summary

- Admin-Wizard zum XLSX-Import aus ASV-BW mit heuristischer Spalten-Erkennung
- Schuljahr-Verwaltung (anlegen, aktivieren, genau eins aktiv via partial unique index)
- Re-Import via ASV-UUID (stabil über Schuljahre) + Fallback (Vorname, Nachname, Klasse)
- 13 neue Rust-Unit-Tests (import::, stammdaten::, db::)
- Admin-only-Route \`/admin/stammdaten\`

## Test plan

- [x] Backend: \`cargo test --no-default-features --lib\` grün
- [x] Frontend: \`npm run check && npm run build\` grün
- [x] CI: Windows-Build grün, Artifact verfügbar
- [x] Smoke: XLSX mit 3 Schüler:innen importieren, erneut importieren (unverändert zählen), Namensänderung testen

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

- [x] **Step 4: CI-Run beobachten**

```bash
sleep 20  # Workflow anstoßen lassen
gh run list -R hibbes/Jiraso-reloaded -L 3
```

Warten bis `conclusion == success`. Falls `failure`: Logs ziehen mit `gh run view <id> --log-failed`, fixen, neu pushen.

- [x] **Step 5: Artifact-URL in PR-Body ergänzen**

Nach grünem Build:

```bash
RUN_ID=$(gh run list -R hibbes/Jiraso-reloaded --branch feat/stammdaten-import -L 1 --json databaseId --jq '.[0].databaseId')
gh pr comment 2 -R hibbes/Jiraso-reloaded --body "Windows-Build grün: https://github.com/hibbes/Jiraso-reloaded/actions/runs/$RUN_ID"
```

- [x] **Step 6: Report an Controller**

Kurzer Status-Report im Agent-Output: erledigte Tasks, PR-URL, Artifact-URL, offene Punkte.

---

## Post-Plan-2 Smoke (manuell auf Schulrechner)

1. Neueste Windows-exe vom PR-Artifact-Zip herunterladen.
2. Als Admin einloggen.
3. Dashboard → „Stammdaten-Import".
4. Neues Schuljahr `2025/26` anlegen + aktivieren.
5. Test-XLSX (z. B. 3 Schüler:innen in 2 Klassen) wählen.
6. Vorschau prüfen, Import bestätigen.
7. Erneut dieselbe Datei importieren → alle als „unverändert".
8. Eine Zeile ändern (z. B. Nachname), erneut importieren → 1 geändert.
9. Ergebnis per SQLite-Browser auf dem Netzlaufwerk verifizieren.
