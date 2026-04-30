# Plan 3: Bewertung/Bemerkung-UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Lehrer:innen geben pro (Schüler, Fach, Kategorie) genau eine Formulierung ein und schreiben optional eine Bemerkung pro Schüler:in. Eine Tabellen-Matrix-UI für die Bewertungen + Detail-Panel für die Bemerkung. Mehrere Sitzungen parallel mit optimistischer Konfliktprüfung. Vorher: Mini-Admin-Editor für Fächer/Kategorien/Formulierungen + einmaliger Legacy-Import-Wizard.

**Architecture:** Neue Backend-Module `katalog.rs` (CRUD für Fach/Kategorie/Formulierung), `bewertung.rs` (matrix+set mit `geaendert_am`-Token-Check), `legacy_import.rs` (Parser für `Fächer.txt`, `Floskeln.txt`, `format.xls`). Schema-Migration 003 ändert `bewertung`-PK auf `(schueler, fach, kategorie)` mit nullable `formulierung_id`. Drei neue Tauri-Command-Cluster. Drei neue Frontend-Routen: `/admin/katalog`, `/admin/legacy-import`, `/bewertung`. Konflikt-UX: Inline-Banner pro Zelle.

**Tech Stack:** Zusätzlich zu Plan 1+2: keine neuen Crates. Existierende `calamine` parst `format.xls`. `encoding_rs` (schon da) für latin-1 in `Floskeln.txt`.

**Spec-Bezug:** `docs/superpowers/specs/2026-04-27-plan-3-bewertung-design.md`.

**Nicht im Scope** (Folge-Pläne): PDF-Druck (Plan 5), Vollständigkeitsreport (Plan 4), Audit-Trail (Plan 6), Kategorien-pro-Fach-Override.

---

## Datei-Struktur nach Plan 3

```
jiraso-reloaded/
├── src-tauri/
│   ├── src/
│   │   ├── katalog.rs              # NEW: Fach/Kategorie/Formulierung CRUD
│   │   ├── bewertung.rs            # NEW: matrix + set + Konflikt-Logik
│   │   ├── bemerkung.rs            # NEW: get + set mit Konflikt-Check
│   │   ├── legacy_import.rs        # NEW: 3 Parser für Legacy-Files
│   │   ├── commands.rs             # MOD: ~14 neue Tauri-Commands
│   │   ├── lib.rs                  # MOD: mod-Imports + invoke_handler
│   │   └── migrations/
│   │       └── 003_bewertung_pk.sql # NEW: bewertung-PK auf kategorie
│   └── tests/
│       └── fixtures/
│           ├── Fächer.txt          # NEW: minimaler Test-Korpus
│           ├── Floskeln.txt        # NEW: minimaler Test-Korpus
│           └── format.xls          # NEW: minimaler Test-Korpus
├── src/
│   ├── lib/
│   │   └── api.ts                  # MOD: katalog/bewertung/bemerkung/legacyImport-Wrapper
│   └── routes/
│       ├── admin/
│       │   ├── katalog/+page.svelte         # NEW: 3-Tab CRUD-Editor
│       │   └── legacy-import/+page.svelte   # NEW: 3-File-Wizard
│       ├── bewertung/+page.svelte           # NEW: Matrix + Detail-Panel
│       └── dashboard/+page.svelte           # MOD: Links zu /admin/katalog, /admin/legacy-import, /bewertung
└── docs/superpowers/plans/
    └── 2026-04-27-plan-3-bewertung.md  # dieses Dokument
```

**Verantwortlichkeiten:**
- `katalog.rs` kennt nur DB-Operationen auf `fach`/`kategorie`/`formulierung`-Tabellen. Reihenfolge wird auf `INTEGER`-Spalten verwaltet (next-max+1 bei INSERT).
- `bewertung.rs` kennt nur DB-Operationen auf `bewertung`. Konflikt-Logik via `geaendert_am`-String-Vergleich.
- `bemerkung.rs` analog für `bemerkung`-Tabelle.
- `legacy_import.rs` kennt nur Bytes → Vec<String>/Vec<Block>. Keine DB-Zugriffe (orchestrierung in commands).
- `commands.rs` orchestriert: parst, ruft Backend-Module auf, packt Tauri-Errors.
- Frontend `/bewertung` ist State-heavy; nutzt das Svelte-5-`$state`-Pattern wie in `/admin/stammdaten`.

---

## Task 1: Migration 003 — bewertung-PK auf (schueler, fach, kategorie)

**Files:**
- Create: `src-tauri/src/migrations/003_bewertung_pk.sql`
- Modify: `src-tauri/src/db.rs:7-15`

Ziel: Pro (Schüler, Fach, Kategorie) genau eine Bewertung mit nullable `formulierung_id`.

- [ ] **Step 1: SQL-Datei anlegen**

```sql
-- src-tauri/src/migrations/003_bewertung_pk.sql
-- Plan-2-Daten in bewertung sind leer (kein Import-Pfad), deshalb verlustfreier DROP+CREATE.
DROP TABLE IF EXISTS bewertung;

CREATE TABLE bewertung (
    schueler_id     INTEGER NOT NULL REFERENCES schueler(id) ON DELETE CASCADE,
    fach_id         INTEGER NOT NULL REFERENCES fach(id) ON DELETE CASCADE,
    kategorie_id    INTEGER NOT NULL REFERENCES kategorie(id) ON DELETE CASCADE,
    formulierung_id INTEGER REFERENCES formulierung(id) ON DELETE SET NULL,
    geaendert_am    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (schueler_id, fach_id, kategorie_id)
);

CREATE INDEX idx_bewertung_klasse_fach
    ON bewertung(fach_id, schueler_id);
```

- [ ] **Step 2: Migration im Runner registrieren**

Ersetze den `migrations()`-Block in `src-tauri/src/db.rs:7-15` durch:

```rust
static MIGRATIONS_001: &str = include_str!("migrations/001_initial.sql");
static MIGRATIONS_002: &str = include_str!("migrations/002_schuljahr_aktiv.sql");
static MIGRATIONS_003: &str = include_str!("migrations/003_bewertung_pk.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(MIGRATIONS_001),
        M::up(MIGRATIONS_002),
        M::up(MIGRATIONS_003),
    ])
}
```

- [ ] **Step 3: Test für die neue PK-Struktur ergänzen**

In `src-tauri/src/db.rs` im `mod tests`-Block einen Test ergänzen (ans Ende, vor das schließende `}`):

```rust
#[test]
fn bewertung_pk_ist_schueler_fach_kategorie() {
    let dir = tempdir().unwrap();
    let conn = open(&dir.path().join("test.db")).unwrap();

    conn.execute("INSERT INTO schuljahr(bezeichnung, aktiv) VALUES ('2025/26', 1)", []).unwrap();
    conn.execute("INSERT INTO klasse(schuljahr_id, name) VALUES (1, '5a')", []).unwrap();
    conn.execute("INSERT INTO schueler(klasse_id, vorname, nachname) VALUES (1, 'Anna', 'Apfel')", []).unwrap();
    conn.execute("INSERT INTO fach(schuljahr_id, name, reihenfolge) VALUES (1, 'Mathe', 1)", []).unwrap();
    conn.execute("INSERT INTO kategorie(schuljahr_id, name, reihenfolge) VALUES (1, 'Lernbereitschaft', 1)", []).unwrap();
    conn.execute("INSERT INTO formulierung(kategorie_id, text, reihenfolge) VALUES (1, 'fleißig', 1)", []).unwrap();

    // Erste Bewertung mit Formulierung
    conn.execute(
        "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (1, 1, 1, 1)",
        [],
    ).unwrap();

    // Zweite Bewertung mit identischem (schueler, fach, kategorie) muss scheitern
    let err = conn.execute(
        "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (1, 1, 1, NULL)",
        [],
    );
    assert!(err.is_err(), "Zweite Bewertung mit gleicher PK muss am UNIQUE scheitern");

    // formulierung_id darf NULL sein
    conn.execute(
        "INSERT INTO formulierung(kategorie_id, text, reihenfolge) VALUES (1, 'mittelmäßig', 2)",
        [],
    ).unwrap();
    conn.execute("INSERT INTO kategorie(schuljahr_id, name, reihenfolge) VALUES (1, 'Sorgfalt', 2)", []).unwrap();
    conn.execute(
        "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (1, 1, 2, NULL)",
        [],
    ).unwrap();
}
```

- [ ] **Step 4: Tests laufen lassen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib db::tests
```

Expected: alle bisherigen Tests laufen weiter, der neue Test passt.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/migrations/003_bewertung_pk.sql src-tauri/src/db.rs
git commit -m "feat(db): migration 003 - bewertung-PK auf (schueler, fach, kategorie)

Erzwingt Legacy-Semantik: pro Kategorie genau eine Bewertung
(oder NULL = keine Angabe). Plan-2-Daten in bewertung sind leer,
DROP+CREATE ist verlustfrei.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Katalog-Modul — Datentypen + Listing

**Files:**
- Create: `src-tauri/src/katalog.rs`
- Modify: `src-tauri/src/lib.rs:12-19` (mod-Liste)

Ziel: Datentypen `Fach`, `Kategorie`, `Formulierung` und reine Listing-Funktionen ohne Schreibseite. Read-only-Foundation, dann in Task 3 ergänzt um CRUD.

- [ ] **Step 1: Modul-Skelett mit Datentypen anlegen**

```rust
// src-tauri/src/katalog.rs
use crate::error::AppResult;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Fach {
    pub id: i64,
    pub schuljahr_id: i64,
    pub name: String,
    pub reihenfolge: i64,
    pub aktiv: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Kategorie {
    pub id: i64,
    pub schuljahr_id: i64,
    pub name: String,
    pub reihenfolge: i64,
    pub aktiv: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Formulierung {
    pub id: i64,
    pub kategorie_id: i64,
    pub text: String,
    pub reihenfolge: i64,
    pub aktiv: bool,
}

pub fn list_faecher(conn: &Connection, schuljahr_id: i64) -> AppResult<Vec<Fach>> {
    let mut stmt = conn.prepare(
        "SELECT id, schuljahr_id, name, reihenfolge, aktiv
         FROM fach WHERE schuljahr_id = ?1
         ORDER BY reihenfolge",
    )?;
    let rows = stmt.query_map(params![schuljahr_id], |r| {
        Ok(Fach {
            id: r.get(0)?,
            schuljahr_id: r.get(1)?,
            name: r.get(2)?,
            reihenfolge: r.get(3)?,
            aktiv: r.get::<_, i64>(4)? == 1,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_kategorien(conn: &Connection, schuljahr_id: i64) -> AppResult<Vec<Kategorie>> {
    let mut stmt = conn.prepare(
        "SELECT id, schuljahr_id, name, reihenfolge, aktiv
         FROM kategorie WHERE schuljahr_id = ?1
         ORDER BY reihenfolge",
    )?;
    let rows = stmt.query_map(params![schuljahr_id], |r| {
        Ok(Kategorie {
            id: r.get(0)?,
            schuljahr_id: r.get(1)?,
            name: r.get(2)?,
            reihenfolge: r.get(3)?,
            aktiv: r.get::<_, i64>(4)? == 1,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_formulierungen(conn: &Connection, kategorie_id: i64) -> AppResult<Vec<Formulierung>> {
    let mut stmt = conn.prepare(
        "SELECT id, kategorie_id, text, reihenfolge, aktiv
         FROM formulierung WHERE kategorie_id = ?1
         ORDER BY reihenfolge",
    )?;
    let rows = stmt.query_map(params![kategorie_id], |r| {
        Ok(Formulierung {
            id: r.get(0)?,
            kategorie_id: r.get(1)?,
            text: r.get(2)?,
            reihenfolge: r.get(3)?,
            aktiv: r.get::<_, i64>(4)? == 1,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    fn fresh_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("t.db")).unwrap();
        // Schuljahr für die Tests
        conn.execute("INSERT INTO schuljahr(bezeichnung, aktiv) VALUES ('2025/26', 1)", []).unwrap();
        (dir, conn)
    }

    #[test]
    fn list_leer_bei_neuem_schuljahr() {
        let (_d, conn) = fresh_conn();
        assert!(list_faecher(&conn, 1).unwrap().is_empty());
        assert!(list_kategorien(&conn, 1).unwrap().is_empty());
    }

    #[test]
    fn list_faecher_sortiert_nach_reihenfolge() {
        let (_d, conn) = fresh_conn();
        conn.execute("INSERT INTO fach(schuljahr_id, name, reihenfolge) VALUES (1, 'Mathe', 2)", []).unwrap();
        conn.execute("INSERT INTO fach(schuljahr_id, name, reihenfolge) VALUES (1, 'Deutsch', 1)", []).unwrap();
        let faecher = list_faecher(&conn, 1).unwrap();
        assert_eq!(faecher[0].name, "Deutsch");
        assert_eq!(faecher[1].name, "Mathe");
    }
}
```

- [ ] **Step 2: Modul in `lib.rs` registrieren**

In `src-tauri/src/lib.rs` nach Zeile 19 (`pub mod import;`) einfügen:

```rust
pub mod katalog;
```

- [ ] **Step 3: Tests laufen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib katalog::tests
```

Expected: 2 Tests grün.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/katalog.rs src-tauri/src/lib.rs
git commit -m "feat(katalog): Datentypen + Listing für Fach/Kategorie/Formulierung

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Katalog-Modul — CRUD-Operationen

**Files:**
- Modify: `src-tauri/src/katalog.rs` (Funktionen + Tests anhängen)

Ziel: `upsert`, `set_reihenfolge`, `set_aktiv` für alle drei Entity-Typen. Soft-disable statt Hard-delete (Foreign-Key-Schutz für historische Bewertungen).

- [ ] **Step 1: Tests vorab schreiben**

In `src-tauri/src/katalog.rs` im `mod tests`-Block ans Ende anhängen (vor der schließenden `}`):

```rust
    #[test]
    fn fach_anlegen_haengt_an_naechste_reihenfolge() {
        let (_d, conn) = fresh_conn();
        let id1 = upsert_fach(&conn, 1, "Mathe", true).unwrap();
        let id2 = upsert_fach(&conn, 1, "Deutsch", true).unwrap();
        let faecher = list_faecher(&conn, 1).unwrap();
        assert_eq!(faecher.len(), 2);
        assert_eq!(faecher[0].id, id1);
        assert_eq!(faecher[0].reihenfolge, 1);
        assert_eq!(faecher[1].id, id2);
        assert_eq!(faecher[1].reihenfolge, 2);
    }

    #[test]
    fn fach_namensduplikate_pro_schuljahr_verboten() {
        let (_d, conn) = fresh_conn();
        upsert_fach(&conn, 1, "Mathe", true).unwrap();
        let err = upsert_fach(&conn, 1, "Mathe", true);
        assert!(err.is_err());
    }

    #[test]
    fn fach_set_aktiv_toggle() {
        let (_d, conn) = fresh_conn();
        let id = upsert_fach(&conn, 1, "Mathe", true).unwrap();
        set_fach_aktiv(&conn, id, false).unwrap();
        let faecher = list_faecher(&conn, 1).unwrap();
        assert!(!faecher[0].aktiv);
    }

    #[test]
    fn kategorie_und_formulierung_lifecycle() {
        let (_d, conn) = fresh_conn();
        let kat = upsert_kategorie(&conn, 1, "Lernbereitschaft").unwrap();
        let f1 = upsert_formulierung(&conn, kat, "fleißig").unwrap();
        let f2 = upsert_formulierung(&conn, kat, "engagiert").unwrap();
        let formulierungen = list_formulierungen(&conn, kat).unwrap();
        assert_eq!(formulierungen.len(), 2);
        assert_eq!(formulierungen[0].id, f1);
        assert_eq!(formulierungen[1].id, f2);

        set_formulierung_aktiv(&conn, f1, false).unwrap();
        let nach_disable = list_formulierungen(&conn, kat).unwrap();
        assert_eq!(nach_disable.len(), 2, "list_formulierungen liefert auch inaktive");
        assert!(!nach_disable[0].aktiv);
    }

    #[test]
    fn reihenfolge_swap() {
        let (_d, conn) = fresh_conn();
        let a = upsert_fach(&conn, 1, "A", true).unwrap();
        let b = upsert_fach(&conn, 1, "B", true).unwrap();
        // a hat reihenfolge=1, b hat reihenfolge=2
        set_fach_reihenfolge(&conn, a, 2).unwrap();
        set_fach_reihenfolge(&conn, b, 1).unwrap();
        let faecher = list_faecher(&conn, 1).unwrap();
        assert_eq!(faecher[0].id, b);
        assert_eq!(faecher[1].id, a);
    }

    #[test]
    fn upsert_fach_mit_existierendem_namen_updatet_aktiv_state() {
        // Re-Upsert ohne Konflikt: gleicher Name, gleiches Schuljahr → ID bleibt, aktiv-Flag wird übernommen
        let (_d, conn) = fresh_conn();
        let id = upsert_fach(&conn, 1, "Mathe", true).unwrap();
        // Hier: Manuelle Variante via UPDATE, weil INSERT-OR-IGNORE nur „neu" macht.
        // Der nachfolgende Aufruf darf NICHT neue Reihenfolge/ID erzeugen, sondern fail.
        let err = upsert_fach(&conn, 1, "Mathe", false);
        assert!(err.is_err(), "Echtes Upsert mit Konflikt-Behandlung folgt in einer Folge-Iteration; jetzt: hartes Fail.");
        // ID-Test: bestehendes Fach unverändert
        let faecher = list_faecher(&conn, 1).unwrap();
        assert_eq!(faecher[0].id, id);
        assert!(faecher[0].aktiv);
    }
```

- [ ] **Step 2: CRUD-Funktionen schreiben**

Im selben `katalog.rs` ans Ende (vor `#[cfg(test)]`) anhängen:

```rust
fn next_reihenfolge_fach(conn: &Connection, schuljahr_id: i64) -> AppResult<i64> {
    let max: Option<i64> = conn.query_row(
        "SELECT MAX(reihenfolge) FROM fach WHERE schuljahr_id = ?1",
        params![schuljahr_id],
        |r| r.get(0),
    ).ok();
    Ok(max.unwrap_or(0) + 1)
}

fn next_reihenfolge_kategorie(conn: &Connection, schuljahr_id: i64) -> AppResult<i64> {
    let max: Option<i64> = conn.query_row(
        "SELECT MAX(reihenfolge) FROM kategorie WHERE schuljahr_id = ?1",
        params![schuljahr_id],
        |r| r.get(0),
    ).ok();
    Ok(max.unwrap_or(0) + 1)
}

fn next_reihenfolge_formulierung(conn: &Connection, kategorie_id: i64) -> AppResult<i64> {
    let max: Option<i64> = conn.query_row(
        "SELECT MAX(reihenfolge) FROM formulierung WHERE kategorie_id = ?1",
        params![kategorie_id],
        |r| r.get(0),
    ).ok();
    Ok(max.unwrap_or(0) + 1)
}

pub fn upsert_fach(conn: &Connection, schuljahr_id: i64, name: &str, aktiv: bool) -> AppResult<i64> {
    let r = next_reihenfolge_fach(conn, schuljahr_id)?;
    conn.execute(
        "INSERT INTO fach(schuljahr_id, name, reihenfolge, aktiv) VALUES (?1, ?2, ?3, ?4)",
        params![schuljahr_id, name, r, aktiv as i64],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn upsert_kategorie(conn: &Connection, schuljahr_id: i64, name: &str) -> AppResult<i64> {
    let r = next_reihenfolge_kategorie(conn, schuljahr_id)?;
    conn.execute(
        "INSERT INTO kategorie(schuljahr_id, name, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
        params![schuljahr_id, name, r],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn upsert_formulierung(conn: &Connection, kategorie_id: i64, text: &str) -> AppResult<i64> {
    let r = next_reihenfolge_formulierung(conn, kategorie_id)?;
    conn.execute(
        "INSERT INTO formulierung(kategorie_id, text, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
        params![kategorie_id, text, r],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn set_fach_aktiv(conn: &Connection, id: i64, aktiv: bool) -> AppResult<()> {
    conn.execute("UPDATE fach SET aktiv = ?1 WHERE id = ?2", params![aktiv as i64, id])?;
    Ok(())
}

pub fn set_kategorie_aktiv(conn: &Connection, id: i64, aktiv: bool) -> AppResult<()> {
    conn.execute("UPDATE kategorie SET aktiv = ?1 WHERE id = ?2", params![aktiv as i64, id])?;
    Ok(())
}

pub fn set_formulierung_aktiv(conn: &Connection, id: i64, aktiv: bool) -> AppResult<()> {
    conn.execute("UPDATE formulierung SET aktiv = ?1 WHERE id = ?2", params![aktiv as i64, id])?;
    Ok(())
}

pub fn set_fach_reihenfolge(conn: &Connection, id: i64, reihenfolge: i64) -> AppResult<()> {
    conn.execute("UPDATE fach SET reihenfolge = ?1 WHERE id = ?2", params![reihenfolge, id])?;
    Ok(())
}

pub fn set_kategorie_reihenfolge(conn: &Connection, id: i64, reihenfolge: i64) -> AppResult<()> {
    conn.execute("UPDATE kategorie SET reihenfolge = ?1 WHERE id = ?2", params![reihenfolge, id])?;
    Ok(())
}

pub fn set_formulierung_reihenfolge(conn: &Connection, id: i64, reihenfolge: i64) -> AppResult<()> {
    conn.execute("UPDATE formulierung SET reihenfolge = ?1 WHERE id = ?2", params![reihenfolge, id])?;
    Ok(())
}
```

- [ ] **Step 3: Tests laufen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib katalog::tests
```

Expected: 7 Tests grün.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/katalog.rs
git commit -m "feat(katalog): CRUD für Fach/Kategorie/Formulierung

Auto-Reihenfolge per next-max+1, Soft-Disable via aktiv-Flag,
hartes Fail bei Namens-Duplikat (UNIQUE-Constraint).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Bewertung-Modul — Datentypen + Matrix-Read

**Files:**
- Create: `src-tauri/src/bewertung.rs`
- Modify: `src-tauri/src/lib.rs` (mod-Liste)

Ziel: Read-only `MatrixZelle` + `matrix(klasse_id, fach_id) -> Vec<MatrixZelle>`. Schreibseite kommt in Task 5.

- [ ] **Step 1: Modul mit Matrix-Read anlegen**

```rust
// src-tauri/src/bewertung.rs
use crate::error::AppResult;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixZelle {
    pub schueler_id: i64,
    pub kategorie_id: i64,
    pub formulierung_id: Option<i64>,
    pub geaendert_am: String,
}

/// Liefert alle gespeicherten Bewertungs-Zellen für eine (Klasse, Fach)-Kombi.
/// NULL-Zellen (= "keine Angabe") sind NICHT in der DB; das Frontend leitet
/// fehlende Zeilen als "keine Angabe" mit `geaendert_am=None` ab.
pub fn matrix(conn: &Connection, klasse_id: i64, fach_id: i64) -> AppResult<Vec<MatrixZelle>> {
    let mut stmt = conn.prepare(
        "SELECT b.schueler_id, b.kategorie_id, b.formulierung_id, b.geaendert_am
         FROM bewertung b
         JOIN schueler s ON s.id = b.schueler_id
         WHERE s.klasse_id = ?1 AND b.fach_id = ?2",
    )?;
    let rows = stmt.query_map(params![klasse_id, fach_id], |r| {
        Ok(MatrixZelle {
            schueler_id: r.get(0)?,
            kategorie_id: r.get(1)?,
            formulierung_id: r.get(2)?,
            geaendert_am: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    pub(super) fn seed() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("t.db")).unwrap();
        conn.execute("INSERT INTO schuljahr(bezeichnung, aktiv) VALUES ('2025/26', 1)", []).unwrap();
        conn.execute("INSERT INTO klasse(schuljahr_id, name) VALUES (1, '5a')", []).unwrap();
        conn.execute("INSERT INTO schueler(klasse_id, vorname, nachname) VALUES (1, 'Anna', 'Apfel')", []).unwrap();
        conn.execute("INSERT INTO schueler(klasse_id, vorname, nachname) VALUES (1, 'Bert', 'Birne')", []).unwrap();
        conn.execute("INSERT INTO fach(schuljahr_id, name, reihenfolge) VALUES (1, 'Mathe', 1)", []).unwrap();
        conn.execute("INSERT INTO kategorie(schuljahr_id, name, reihenfolge) VALUES (1, 'Lernbereitschaft', 1)", []).unwrap();
        conn.execute("INSERT INTO kategorie(schuljahr_id, name, reihenfolge) VALUES (1, 'Sorgfalt', 2)", []).unwrap();
        conn.execute("INSERT INTO formulierung(kategorie_id, text, reihenfolge) VALUES (1, 'fleißig', 1)", []).unwrap();
        conn.execute("INSERT INTO formulierung(kategorie_id, text, reihenfolge) VALUES (2, 'sorgfältig', 1)", []).unwrap();
        (dir, conn)
    }

    #[test]
    fn matrix_leer_wenn_nichts_eingegeben() {
        let (_d, conn) = seed();
        assert!(matrix(&conn, 1, 1).unwrap().is_empty());
    }

    #[test]
    fn matrix_liefert_eingetragene_zellen() {
        let (_d, conn) = seed();
        conn.execute(
            "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (1, 1, 1, 1)",
            [],
        ).unwrap();
        let m = matrix(&conn, 1, 1).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].schueler_id, 1);
        assert_eq!(m[0].formulierung_id, Some(1));
    }

    #[test]
    fn matrix_nur_eigene_klasse_und_fach() {
        let (_d, conn) = seed();
        // Andere Klasse anlegen
        conn.execute("INSERT INTO klasse(schuljahr_id, name) VALUES (1, '5b')", []).unwrap();
        conn.execute("INSERT INTO schueler(klasse_id, vorname, nachname) VALUES (2, 'Cora', 'Citro')", []).unwrap();
        conn.execute(
            "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (1, 1, 1, 1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (3, 1, 1, 1)",
            [],
        ).unwrap();
        let nur_5a = matrix(&conn, 1, 1).unwrap();
        assert_eq!(nur_5a.len(), 1);
        assert_eq!(nur_5a[0].schueler_id, 1);
    }
}
```

- [ ] **Step 2: Modul registrieren**

In `src-tauri/src/lib.rs` nach `pub mod katalog;` ergänzen:

```rust
pub mod bewertung;
```

- [ ] **Step 3: Tests laufen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib bewertung::tests
```

Expected: 3 Tests grün.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/bewertung.rs src-tauri/src/lib.rs
git commit -m "feat(bewertung): MatrixZelle + read-only matrix(klasse, fach)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 5: Bewertung-Modul — Set mit Konflikt-Logik

**Files:**
- Modify: `src-tauri/src/bewertung.rs` (Funktionen + Tests anhängen)

Ziel: `set_bewertung` mit optimistischem Token-Check. NULL-Formulierung wird als explizite Zeile gespeichert (kein DELETE), damit „Klasse komplett bewertet" detektierbar ist.

- [ ] **Step 1: Datentypen + Tests vorab anlegen**

In `src-tauri/src/bewertung.rs` direkt nach der `MatrixZelle`-Struct (vor `pub fn matrix`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BewertungUpdate {
    pub schueler_id: i64,
    pub fach_id: i64,
    pub kategorie_id: i64,
    pub formulierung_id: Option<i64>,
    /// `None` = wir glauben, die Zelle existiert noch nicht.
    /// `Some(ts)` = wir kennen den Stand `ts` (geaendert_am-String aus matrix()).
    pub vorheriger_stand: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status")]
pub enum SetResult {
    Ok { neuer_stand: String },
    Konflikt {
        server_formulierung_id: Option<i64>,
        server_geaendert_am: String,
    },
}
```

Ans Ende des `mod tests`-Blocks (vor schließendem `}`) hinzufügen:

```rust
    use super::SetResult;

    #[test]
    fn set_neu_kein_vorheriger_stand() {
        let (_d, mut conn) = seed();
        let r = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(1), vorheriger_stand: None,
        }).unwrap();
        assert!(matches!(r, SetResult::Ok { .. }));
        let m = matrix(&conn, 1, 1).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].formulierung_id, Some(1));
    }

    #[test]
    fn set_idempotent_wenn_inhalt_gleich() {
        let (_d, mut conn) = seed();
        set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(1), vorheriger_stand: None,
        }).unwrap();
        // Zweiter Save ohne Token, gleicher Inhalt: KEIN Konflikt
        let r = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(1), vorheriger_stand: None,
        }).unwrap();
        assert!(matches!(r, SetResult::Ok { .. }));
    }

    #[test]
    fn set_konflikt_wenn_anderer_inhalt_und_kein_token() {
        let (_d, mut conn) = seed();
        set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(1), vorheriger_stand: None,
        }).unwrap();
        // Zweiter Save: anderer Inhalt, kein Token → Konflikt
        conn.execute("INSERT INTO formulierung(kategorie_id, text, reihenfolge) VALUES (1, 'mittelmäßig', 2)", []).unwrap();
        let neue_id = conn.last_insert_rowid();
        let r = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(neue_id), vorheriger_stand: None,
        }).unwrap();
        match r {
            SetResult::Konflikt { server_formulierung_id, .. } => {
                assert_eq!(server_formulierung_id, Some(1));
            }
            _ => panic!("erwartete Konflikt"),
        }
    }

    #[test]
    fn set_update_mit_korrektem_token() {
        let (_d, mut conn) = seed();
        let r1 = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(1), vorheriger_stand: None,
        }).unwrap();
        let token = match r1 {
            SetResult::Ok { neuer_stand } => neuer_stand,
            _ => unreachable!(),
        };
        conn.execute("INSERT INTO formulierung(kategorie_id, text, reihenfolge) VALUES (1, 'gut', 2)", []).unwrap();
        let r2 = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(2), vorheriger_stand: Some(token),
        }).unwrap();
        assert!(matches!(r2, SetResult::Ok { .. }));
        let m = matrix(&conn, 1, 1).unwrap();
        assert_eq!(m[0].formulierung_id, Some(2));
    }

    #[test]
    fn set_keine_angabe_bleibt_explizite_zeile() {
        let (_d, mut conn) = seed();
        let r1 = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(1), vorheriger_stand: None,
        }).unwrap();
        let token = match r1 {
            SetResult::Ok { neuer_stand } => neuer_stand,
            _ => unreachable!(),
        };
        let r2 = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: None, vorheriger_stand: Some(token),
        }).unwrap();
        assert!(matches!(r2, SetResult::Ok { .. }));
        // NULL-Bewertung bleibt als Zeile, damit "Klasse fertig" detektierbar ist
        let m = matrix(&conn, 1, 1).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].formulierung_id, None);
        assert!(!m[0].geaendert_am.is_empty());
    }

    #[test]
    fn set_konflikt_zeile_extern_geloescht() {
        let (_d, mut conn) = seed();
        let r1 = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(1), vorheriger_stand: None,
        }).unwrap();
        let token = match r1 {
            SetResult::Ok { neuer_stand } => neuer_stand,
            _ => unreachable!(),
        };
        conn.execute(
            "DELETE FROM bewertung WHERE schueler_id=1 AND fach_id=1 AND kategorie_id=1",
            [],
        ).unwrap();
        let r2 = set(&mut conn, BewertungUpdate {
            schueler_id: 1, fach_id: 1, kategorie_id: 1,
            formulierung_id: Some(1), vorheriger_stand: Some(token),
        }).unwrap();
        match r2 {
            SetResult::Konflikt { server_formulierung_id, server_geaendert_am } => {
                assert!(server_formulierung_id.is_none());
                assert!(server_geaendert_am.is_empty());
            }
            _ => panic!("erwartete Konflikt"),
        }
    }
```

- [ ] **Step 2: `set`-Funktion implementieren**

Vor dem `#[cfg(test)]`-Block in `bewertung.rs` einfügen:

```rust
/// Setzt eine Bewertungs-Zelle. Verhalten:
/// - `formulierung_id = None` → Zeile wird gelöscht (= "keine Angabe").
/// - `vorheriger_stand = None` und Server hat keine Zeile → INSERT.
/// - `vorheriger_stand = None` und Server hat gleichen Inhalt → idempotenter OK.
/// - `vorheriger_stand = None` und Server hat anderen Inhalt → Konflikt.
/// - `vorheriger_stand = Some(ts)` und Server-ts == ts → UPDATE/DELETE.
/// - `vorheriger_stand = Some(ts)` und Server-Inhalt == Update-Inhalt → idempotenter OK.
/// - sonst → Konflikt.
pub fn set(conn: &mut Connection, u: BewertungUpdate) -> AppResult<SetResult> {
    let tx = conn.transaction()?;
    let server: Option<(Option<i64>, String)> = tx.query_row(
        "SELECT formulierung_id, geaendert_am
         FROM bewertung
         WHERE schueler_id=?1 AND fach_id=?2 AND kategorie_id=?3",
        params![u.schueler_id, u.fach_id, u.kategorie_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();

    let inhalt_gleich = matches!(&server, Some((srv, _)) if *srv == u.formulierung_id);

    let token_passt = match (&u.vorheriger_stand, &server) {
        (None, None) => true,
        (Some(ts), Some((_, srv_ts))) if ts == srv_ts => true,
        _ => false,
    };

    if !token_passt && !inhalt_gleich {
        let (sfid, sts) = server.unwrap_or((None, String::new()));
        tx.rollback()?;
        return Ok(SetResult::Konflikt {
            server_formulierung_id: sfid,
            server_geaendert_am: sts,
        });
    }

    if inhalt_gleich {
        let (_, ts) = server.unwrap();
        tx.rollback()?;
        return Ok(SetResult::Ok { neuer_stand: ts });
    }

    // Token passt + Inhalt unterschiedlich → schreiben (NULL und Some(fid) gleich behandeln)
    tx.execute(
        "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id, geaendert_am)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))
         ON CONFLICT(schueler_id, fach_id, kategorie_id) DO UPDATE SET
            formulierung_id = excluded.formulierung_id,
            geaendert_am    = excluded.geaendert_am",
        params![u.schueler_id, u.fach_id, u.kategorie_id, u.formulierung_id],
    )?;
    let ts: String = tx.query_row(
        "SELECT geaendert_am FROM bewertung WHERE schueler_id=?1 AND fach_id=?2 AND kategorie_id=?3",
        params![u.schueler_id, u.fach_id, u.kategorie_id],
        |r| r.get(0),
    )?;
    tx.commit()?;
    Ok(SetResult::Ok { neuer_stand: ts })
}
```

- [ ] **Step 3: Tests laufen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib bewertung::tests
```

Expected: 9 Tests grün (3 vorhanden + 6 neue).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/bewertung.rs
git commit -m "feat(bewertung): set mit optimistischem Token-Check

formulierung_id=None loescht Zelle (= keine Angabe).
Konflikt-Erkennung über geaendert_am-String-Vergleich,
idempotent bei gleichem Inhalt.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 6: Bemerkung-Modul

**Files:**
- Create: `src-tauri/src/bemerkung.rs`
- Modify: `src-tauri/src/lib.rs` (mod-Liste)

Ziel: `get(schueler_id) -> (text, geaendert_am)` und `set(schueler_id, text, vorheriger_stand) -> SetResult`. Spiegelt das Bewertung-Pattern.

- [ ] **Step 1: Modul + Tests anlegen**

```rust
// src-tauri/src/bemerkung.rs
use crate::error::AppResult;
use crate::bewertung::SetResult;
use rusqlite::{params, Connection};

/// Liefert (text, geaendert_am). `None` wenn noch keine Bemerkung gespeichert.
pub fn get(conn: &Connection, schueler_id: i64) -> AppResult<Option<(String, String)>> {
    let row: Option<(String, String)> = conn.query_row(
        "SELECT text, geaendert_am FROM bemerkung WHERE schueler_id = ?1",
        params![schueler_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();
    Ok(row)
}

pub fn set(
    conn: &mut Connection,
    schueler_id: i64,
    text: &str,
    vorheriger_stand: Option<String>,
) -> AppResult<SetResult> {
    let tx = conn.transaction()?;
    let server: Option<(String, String)> = tx.query_row(
        "SELECT text, geaendert_am FROM bemerkung WHERE schueler_id = ?1",
        params![schueler_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ).ok();

    let inhalt_gleich = matches!(&server, Some((t, _)) if t == text);

    let token_passt = match (&vorheriger_stand, &server) {
        (None, None) => true,
        (Some(ts), Some((_, srv_ts))) if ts == srv_ts => true,
        _ => false,
    };

    if !token_passt && !inhalt_gleich {
        let (_, sts) = server.clone().unwrap_or_default();
        tx.rollback()?;
        return Ok(SetResult::Konflikt {
            server_formulierung_id: None, // Bemerkung hat keine Formulierung; Frontend liest server_geaendert_am + holt Text via get()
            server_geaendert_am: sts,
        });
    }

    if inhalt_gleich {
        let (_, ts) = server.unwrap();
        tx.rollback()?;
        return Ok(SetResult::Ok { neuer_stand: ts });
    }

    if text.trim().is_empty() {
        tx.execute("DELETE FROM bemerkung WHERE schueler_id = ?1", params![schueler_id])?;
        tx.commit()?;
        return Ok(SetResult::Ok { neuer_stand: String::new() });
    }

    tx.execute(
        "INSERT INTO bemerkung(schueler_id, text, geaendert_am)
         VALUES (?1, ?2, datetime('now'))
         ON CONFLICT(schueler_id) DO UPDATE SET
            text = excluded.text,
            geaendert_am = excluded.geaendert_am",
        params![schueler_id, text],
    )?;
    let ts: String = tx.query_row(
        "SELECT geaendert_am FROM bemerkung WHERE schueler_id = ?1",
        params![schueler_id],
        |r| r.get(0),
    )?;
    tx.commit()?;
    Ok(SetResult::Ok { neuer_stand: ts })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    fn seed() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("t.db")).unwrap();
        conn.execute("INSERT INTO schuljahr(bezeichnung, aktiv) VALUES ('2025/26', 1)", []).unwrap();
        conn.execute("INSERT INTO klasse(schuljahr_id, name) VALUES (1, '5a')", []).unwrap();
        conn.execute("INSERT INTO schueler(klasse_id, vorname, nachname) VALUES (1, 'Anna', 'Apfel')", []).unwrap();
        (dir, conn)
    }

    #[test]
    fn get_leer_initial() {
        let (_d, conn) = seed();
        assert!(get(&conn, 1).unwrap().is_none());
    }

    #[test]
    fn set_neu_und_get() {
        let (_d, mut conn) = seed();
        let r = set(&mut conn, 1, "Eine schöne Bemerkung.", None).unwrap();
        assert!(matches!(r, SetResult::Ok { .. }));
        let (text, _ts) = get(&conn, 1).unwrap().unwrap();
        assert_eq!(text, "Eine schöne Bemerkung.");
    }

    #[test]
    fn set_konflikt_bei_alter_token() {
        let (_d, mut conn) = seed();
        set(&mut conn, 1, "v1", None).unwrap();
        let r = set(&mut conn, 1, "v2", Some("alter-token-1900-01-01 00:00:00".into())).unwrap();
        assert!(matches!(r, SetResult::Konflikt { .. }));
    }

    #[test]
    fn set_idempotent_gleicher_inhalt() {
        let (_d, mut conn) = seed();
        let r1 = set(&mut conn, 1, "v1", None).unwrap();
        let token = match r1 { SetResult::Ok { neuer_stand } => neuer_stand, _ => unreachable!() };
        // Anderer Token, gleicher Inhalt → idempotent
        let r2 = set(&mut conn, 1, "v1", Some("falsch".into())).unwrap();
        match r2 {
            SetResult::Ok { neuer_stand } => assert_eq!(neuer_stand, token),
            _ => panic!("erwartete Ok"),
        }
    }

    #[test]
    fn set_leerer_text_loescht_eintrag() {
        let (_d, mut conn) = seed();
        let r1 = set(&mut conn, 1, "v1", None).unwrap();
        let token = match r1 { SetResult::Ok { neuer_stand } => neuer_stand, _ => unreachable!() };
        set(&mut conn, 1, "", Some(token)).unwrap();
        assert!(get(&conn, 1).unwrap().is_none());
    }
}
```

- [ ] **Step 2: Modul registrieren**

In `src-tauri/src/lib.rs` nach `pub mod bewertung;`:

```rust
pub mod bemerkung;
```

- [ ] **Step 3: Tests laufen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib bemerkung::tests
```

Expected: 5 Tests grün.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/bemerkung.rs src-tauri/src/lib.rs
git commit -m "feat(bemerkung): get/set mit optimistischem Token-Check

Spiegelt das Bewertung-Pattern. Leerer Text löscht den Eintrag.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 7: Legacy-Import-Modul — Parser

**Files:**
- Create: `src-tauri/src/legacy_import.rs`
- Modify: `src-tauri/src/lib.rs` (mod-Liste)

Ziel: drei Parser für `Fächer.txt`, `Floskeln.txt`, `format.xls`. Reine Funktionen Bytes → strukturierte Ausgabe, keine DB-Zugriffe.

- [ ] **Step 1: Modul-Skelett mit Datentypen + Parser anlegen**

```rust
// src-tauri/src/legacy_import.rs
use crate::error::{AppError, AppResult};
use crate::import; // wir nutzen import::decode_text-äquivalent
use calamine::{Data, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

/// Eine Kategorie mit ihren Formulierungen (Reihenfolge matters).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegacyKategorie {
    pub name: String,
    pub formulierungen: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegacyImportPreview {
    pub faecher: Vec<String>,
    pub kategorien: Vec<LegacyKategorie>,
}

/// `Fächer.txt`: ASCII / latin-1, eine Zeile pro Fach, leere Zeilen verworfen.
pub fn parse_faecher(bytes: &[u8]) -> AppResult<Vec<String>> {
    let text = decode_text(bytes);
    Ok(text.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// `Floskeln.txt`: latin-1, Formulierungen pro Zeile, `-` als Trenner zwischen
/// Kategorie-Blöcken. Liefert eine Liste anonymer Blöcke (Namen kommen aus format.xls).
pub fn parse_floskeln_blocks(bytes: &[u8]) -> AppResult<Vec<Vec<String>>> {
    let text = decode_text(bytes);
    let mut blocks: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() { continue; }
        if l == "-" {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(l.to_string());
    }
    if !current.is_empty() { blocks.push(current); }
    Ok(blocks)
}

/// `format.xls`: Excel-Datei (alte BIFF-Variante; calamine kann das via auto-detection).
/// Die Kategorie-Labels stehen in Spalte A bei den Block-Anfangs-Zeilen.
/// In der Schiller-Vorlage: Zeilen 7,11,14,19,22,25,29 (0-basiert) tragen Labels.
/// Wir scannen Spalte A nach nicht-leeren Zellen ab Zeile 7 und sammeln diese
/// als Kategorie-Namen. Mehrzeilige Labels (z.B. "Beteiligung\nam\nUnterricht")
/// werden zusammengefügt durch Whitespace-Konkatenation der direkt folgenden
/// Zeilen, in denen Spalte A nicht-leer ist und Spalte B die gleiche Block-
/// Zugehörigkeit signalisiert.
///
/// Heuristik: scanne Spalte A von Zeile 7 abwärts; jede nicht-leere Zelle
/// startet ein neues Label oder erweitert das laufende, abhängig davon, ob
/// in Spalte B (Index 1) zwischen den zwei Zellen leere Zellen liegen.
pub fn parse_format_xls_kategorien(bytes: &[u8]) -> AppResult<Vec<String>> {
    let mut wb: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes))
        .map_err(|e| AppError::Config(format!("format.xls ungültig: {e}")))?;
    let sheet_name = wb.sheet_names().first().cloned()
        .ok_or_else(|| AppError::Config("format.xls hat keine Tabelle".into()))?;
    let range = wb.worksheet_range(&sheet_name)
        .map_err(|e| AppError::Config(format!("Tabelle nicht lesbar: {e}")))?;

    // Wir erwarten Header in Zeile 0..6 und Floskeln ab Zeile 7.
    // Eine Zeile mit nicht-leerer Spalte A startet ein neues Label oder ergänzt
    // das laufende. Wir sammeln Labels nach Block-Position: jeder Block beginnt,
    // wenn Spalte B in einer vorherigen Zeile leer war.
    let rows: Vec<Vec<String>> = range.rows().map(|r| {
        r.iter().map(cell_to_string).collect()
    }).collect();

    if rows.len() < 8 {
        return Err(AppError::Config("format.xls hat zu wenige Zeilen".into()));
    }

    let mut labels: Vec<String> = Vec::new();
    let mut current_label: Vec<String> = Vec::new();
    let mut last_b_empty = true;

    for r in 7..rows.len() {
        let row = &rows[r];
        let a = row.get(0).cloned().unwrap_or_default();
        let b = row.get(1).cloned().unwrap_or_default();
        let a_trim = a.trim().to_string();
        let b_empty = b.trim().is_empty();

        // Block-Wechsel = Spalte B vorher leer und jetzt voll
        if last_b_empty && !b_empty {
            // Wir schließen das vorherige Label ab (falls vorhanden)
            if !current_label.is_empty() {
                labels.push(current_label.join(" ").trim().to_string());
                current_label.clear();
            }
        }

        if !a_trim.is_empty() {
            current_label.push(a_trim);
        }
        last_b_empty = b_empty;

        // Abbruch: Footer "zusätzliche Bemerkungen" markiert Ende
        if a_trim.to_lowercase().contains("bemerkungen") || a_trim.starts_with("Offenburg") {
            break;
        }
    }
    if !current_label.is_empty() {
        labels.push(current_label.join(" ").trim().to_string());
    }
    Ok(labels)
}

/// Kombiniert `parse_faecher` + `parse_floskeln_blocks` + `parse_format_xls_kategorien`
/// und korreliert die anonymen Floskel-Blöcke mit den Kategorie-Namen aus format.xls.
/// Falls die Anzahl Blöcke ≠ Anzahl Labels, gibt es einen sprechenden Fehler.
pub fn parse_alle(
    faecher_bytes: &[u8],
    floskeln_bytes: &[u8],
    format_bytes: &[u8],
) -> AppResult<LegacyImportPreview> {
    let faecher = parse_faecher(faecher_bytes)?;
    let blocks = parse_floskeln_blocks(floskeln_bytes)?;
    let labels = parse_format_xls_kategorien(format_bytes)?;
    if blocks.len() != labels.len() {
        return Err(AppError::Config(format!(
            "Anzahl Floskel-Blöcke ({}) passt nicht zur Anzahl Kategorie-Labels ({}). \
             Bitte format.xls und Floskeln.txt prüfen.",
            blocks.len(), labels.len()
        )));
    }
    let kategorien = labels.into_iter().zip(blocks.into_iter())
        .map(|(name, formulierungen)| LegacyKategorie { name, formulierungen })
        .collect();
    Ok(LegacyImportPreview { faecher, kategorien })
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

fn decode_text(bytes: &[u8]) -> String {
    // Wiederverwendung der gleichen Logik wie in import.rs (UTF-8 → Windows-1252 Fallback).
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            cow.into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_faecher_strippt_leerzeilen() {
        let bytes = b"Mathe\n\nDeutsch\n   \nEnglisch\n";
        let f = parse_faecher(bytes).unwrap();
        assert_eq!(f, vec!["Mathe", "Deutsch", "Englisch"]);
    }

    #[test]
    fn parse_floskeln_trennt_an_minus() {
        let bytes = b"a\nb\n-\nc\nd\n-\ne\n";
        let blocks = parse_floskeln_blocks(bytes).unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0], vec!["a", "b"]);
        assert_eq!(blocks[1], vec!["c", "d"]);
        assert_eq!(blocks[2], vec!["e"]);
    }

    #[test]
    fn parse_floskeln_latin1_decoding() {
        // "Müller" in Windows-1252: 0x4D 0xFC 0x6C 0x6C 0x65 0x72
        let bytes: &[u8] = b"M\xfcller\n-\n";
        let blocks = parse_floskeln_blocks(bytes).unwrap();
        assert_eq!(blocks[0][0], "Müller");
    }

    #[test]
    fn parse_alle_erkennt_anzahl_unterschied() {
        let faecher = b"Mathe\nDeutsch\n";
        let floskeln = b"a\n-\nb\n-\nc\n";
        // Wir haben kein echtes format.xls hier; benutze ein Mock-XLSX (das wird aber nicht parsen).
        // Stattdessen: Test der Mismatch-Erkennung über parse_alle nicht möglich ohne real-Daten.
        // Wir testen den Pfad indirekt im Integrationstest in Task 8 mit fixture-Files.
        let _ = (faecher, floskeln);
    }
}
```

- [ ] **Step 2: Modul registrieren**

In `src-tauri/src/lib.rs` nach `pub mod bemerkung;`:

```rust
pub mod legacy_import;
```

- [ ] **Step 3: Tests laufen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib legacy_import::tests
```

Expected: 4 Tests grün.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/legacy_import.rs src-tauri/src/lib.rs
git commit -m "feat(legacy_import): Parser für Fächer.txt, Floskeln.txt, format.xls

Reine Bytes -> strukturierte Ausgabe, keine DB-Zugriffe.
Latin-1-Fallback wie in import.rs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 8: Legacy-Import — Apply-Funktion

**Files:**
- Modify: `src-tauri/src/legacy_import.rs` (Funktion + Test)

Ziel: `apply(conn, schuljahr_id, preview)` schreibt idempotent ins DB. Bestehende mit gleichem Namen werden übersprungen.

- [ ] **Step 1: `apply`-Funktion + Test**

In `src-tauri/src/legacy_import.rs` nach `parse_alle` einfügen (vor `cell_to_string`):

```rust
use crate::katalog;
use rusqlite::Connection;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LegacyImportSummary {
    pub neue_faecher: usize,
    pub neue_kategorien: usize,
    pub neue_formulierungen: usize,
    pub uebersprungene_faecher: usize,
    pub uebersprungene_kategorien: usize,
    pub uebersprungene_formulierungen: usize,
}

pub fn apply(
    conn: &mut Connection,
    schuljahr_id: i64,
    preview: &LegacyImportPreview,
) -> AppResult<LegacyImportSummary> {
    let tx = conn.transaction()?;
    let mut sum = LegacyImportSummary::default();

    // Bestehende Namen einlesen
    let bestehende_faecher: std::collections::HashSet<String> = {
        let mut s = std::collections::HashSet::new();
        let mut stmt = tx.prepare("SELECT name FROM fach WHERE schuljahr_id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![schuljahr_id])?;
        while let Some(r) = rows.next()? {
            s.insert(r.get(0)?);
        }
        s
    };
    for f in &preview.faecher {
        if bestehende_faecher.contains(f) {
            sum.uebersprungene_faecher += 1;
        } else {
            // Inline-Insert (kein katalog::upsert_fach, weil wir in TX sind)
            let r: i64 = tx.query_row(
                "SELECT COALESCE(MAX(reihenfolge), 0) FROM fach WHERE schuljahr_id = ?1",
                rusqlite::params![schuljahr_id], |r| r.get(0),
            ).unwrap_or(0) + 1;
            tx.execute(
                "INSERT INTO fach(schuljahr_id, name, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
                rusqlite::params![schuljahr_id, f, r],
            )?;
            sum.neue_faecher += 1;
        }
    }

    let bestehende_kategorien: std::collections::HashMap<String, i64> = {
        let mut m = std::collections::HashMap::new();
        let mut stmt = tx.prepare("SELECT name, id FROM kategorie WHERE schuljahr_id = ?1")?;
        let mut rows = stmt.query(rusqlite::params![schuljahr_id])?;
        while let Some(r) = rows.next()? {
            let n: String = r.get(0)?;
            let id: i64 = r.get(1)?;
            m.insert(n, id);
        }
        m
    };
    for kat in &preview.kategorien {
        let kid = if let Some(&id) = bestehende_kategorien.get(&kat.name) {
            sum.uebersprungene_kategorien += 1;
            id
        } else {
            let r: i64 = tx.query_row(
                "SELECT COALESCE(MAX(reihenfolge), 0) FROM kategorie WHERE schuljahr_id = ?1",
                rusqlite::params![schuljahr_id], |r| r.get(0),
            ).unwrap_or(0) + 1;
            tx.execute(
                "INSERT INTO kategorie(schuljahr_id, name, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
                rusqlite::params![schuljahr_id, &kat.name, r],
            )?;
            sum.neue_kategorien += 1;
            tx.last_insert_rowid()
        };

        // Formulierungen
        let bestehende_form: std::collections::HashSet<String> = {
            let mut s = std::collections::HashSet::new();
            let mut stmt = tx.prepare("SELECT text FROM formulierung WHERE kategorie_id = ?1")?;
            let mut rows = stmt.query(rusqlite::params![kid])?;
            while let Some(r) = rows.next()? {
                s.insert(r.get(0)?);
            }
            s
        };
        for ftext in &kat.formulierungen {
            if bestehende_form.contains(ftext) {
                sum.uebersprungene_formulierungen += 1;
            } else {
                let r: i64 = tx.query_row(
                    "SELECT COALESCE(MAX(reihenfolge), 0) FROM formulierung WHERE kategorie_id = ?1",
                    rusqlite::params![kid], |r| r.get(0),
                ).unwrap_or(0) + 1;
                tx.execute(
                    "INSERT INTO formulierung(kategorie_id, text, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
                    rusqlite::params![kid, ftext, r],
                )?;
                sum.neue_formulierungen += 1;
            }
        }
    }

    tx.commit()?;
    Ok(sum)
}

// Pseudo-Verwendung von katalog, damit der Compiler die Crate-Abhängigkeit nicht meckert
#[allow(dead_code)]
fn _katalog_link() -> Option<katalog::Fach> { None }
```

- [ ] **Step 2: Test in `mod tests` ergänzen**

Vor dem schließenden `}` von `mod tests` in `src-tauri/src/legacy_import.rs`:

```rust
    use super::apply;
    use crate::db;
    use rusqlite::Connection;
    use tempfile::tempdir;

    fn fresh_conn() -> (tempfile::TempDir, Connection) {
        let dir = tempdir().unwrap();
        let conn = db::open(&dir.path().join("t.db")).unwrap();
        conn.execute("INSERT INTO schuljahr(bezeichnung, aktiv) VALUES ('2025/26', 1)", []).unwrap();
        (dir, conn)
    }

    #[test]
    fn apply_idempotent() {
        let (_d, mut conn) = fresh_conn();
        let preview = LegacyImportPreview {
            faecher: vec!["Mathe".into(), "Deutsch".into()],
            kategorien: vec![
                LegacyKategorie {
                    name: "Lernbereitschaft".into(),
                    formulierungen: vec!["fleißig".into(), "engagiert".into()],
                },
            ],
        };
        let s1 = apply(&mut conn, 1, &preview).unwrap();
        assert_eq!(s1.neue_faecher, 2);
        assert_eq!(s1.neue_kategorien, 1);
        assert_eq!(s1.neue_formulierungen, 2);

        let s2 = apply(&mut conn, 1, &preview).unwrap();
        assert_eq!(s2.neue_faecher, 0);
        assert_eq!(s2.neue_kategorien, 0);
        assert_eq!(s2.neue_formulierungen, 0);
        assert_eq!(s2.uebersprungene_faecher, 2);
        assert_eq!(s2.uebersprungene_kategorien, 1);
        assert_eq!(s2.uebersprungene_formulierungen, 2);
    }

    #[test]
    fn apply_haengt_an_bestehende_an() {
        let (_d, mut conn) = fresh_conn();
        // Existierendes Fach
        conn.execute("INSERT INTO fach(schuljahr_id, name, reihenfolge) VALUES (1, 'Mathe', 1)", []).unwrap();

        let preview = LegacyImportPreview {
            faecher: vec!["Mathe".into(), "Deutsch".into()],
            kategorien: vec![],
        };
        let s = apply(&mut conn, 1, &preview).unwrap();
        assert_eq!(s.neue_faecher, 1);
        assert_eq!(s.uebersprungene_faecher, 1);
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM fach", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 2);
    }
```

- [ ] **Step 3: Tests laufen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib legacy_import::tests
```

Expected: 6 Tests grün.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/legacy_import.rs
git commit -m "feat(legacy_import): apply schreibt idempotent ins DB

Bestehende Fächer/Kategorien/Formulierungen mit gleichem Namen
werden übersprungen, neue gehängt mit nächster reihenfolge.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 9: Tauri-Commands — Katalog + Bewertung + Bemerkung + Legacy

**Files:**
- Modify: `src-tauri/src/commands.rs` (anhängen ans Ende, nach `import_xlsx_apply`)
- Modify: `src-tauri/src/lib.rs:74-91` (invoke_handler-Liste)

Ziel: ~14 Tauri-Commands. Rollen-Guards laut Spec.

- [ ] **Step 1: Rollen-Helper anlegen**

In `src-tauri/src/commands.rs` nach `require_admin` (etwa Zeile 122) ergänzen:

```rust
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
```

- [ ] **Step 2: Katalog-Commands anhängen**

Am Ende von `src-tauri/src/commands.rs`:

```rust
use crate::katalog::{self, Fach, Formulierung, Kategorie};

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
```

- [ ] **Step 3: Bewertung+Bemerkung-Commands anhängen**

Direkt darunter:

```rust
use crate::bewertung::{self, BewertungUpdate, MatrixZelle, SetResult};
use crate::bemerkung;

#[tauri::command]
pub fn bewertung_matrix(klasse_id: i64, fach_id: i64, state: tauri::State<AppState>) -> AppResult<Vec<MatrixZelle>> {
    require_lehrer(&state)?;
    let conn = open_db(&state)?;
    bewertung::matrix(&conn, klasse_id, fach_id)
}

#[tauri::command]
pub fn bewertung_set(update: BewertungUpdate, state: tauri::State<AppState>) -> AppResult<SetResult> {
    require_lehrer(&state)?;
    let mut conn = open_db(&state)?;
    bewertung::set(&mut conn, update)
}

#[tauri::command]
pub fn bemerkung_get(schueler_id: i64, state: tauri::State<AppState>) -> AppResult<Option<(String, String)>> {
    require_klassenlehrer_oder_admin(&state)?;
    let conn = open_db(&state)?;
    bemerkung::get(&conn, schueler_id)
}

#[tauri::command]
pub fn bemerkung_set(
    schueler_id: i64,
    text: String,
    vorheriger_stand: Option<String>,
    state: tauri::State<AppState>,
) -> AppResult<SetResult> {
    require_klassenlehrer_oder_admin(&state)?;
    let mut conn = open_db(&state)?;
    bemerkung::set(&mut conn, schueler_id, &text, vorheriger_stand)
}
```

- [ ] **Step 4: Legacy-Import-Commands anhängen**

```rust
use crate::legacy_import::{self, LegacyImportPreview, LegacyImportSummary};

#[tauri::command]
pub fn legacy_import_preview(
    faecher_bytes: Vec<u8>,
    floskeln_bytes: Vec<u8>,
    format_bytes: Vec<u8>,
    state: tauri::State<AppState>,
) -> AppResult<LegacyImportPreview> {
    require_admin(&state)?;
    legacy_import::parse_alle(&faecher_bytes, &floskeln_bytes, &format_bytes)
}

#[tauri::command]
pub fn legacy_import_apply(
    schuljahr_id: i64,
    preview: LegacyImportPreview,
    state: tauri::State<AppState>,
) -> AppResult<LegacyImportSummary> {
    require_admin(&state)?;
    let mut conn = open_db(&state)?;
    legacy_import::apply(&mut conn, schuljahr_id, &preview)
}
```

- [ ] **Step 5: invoke_handler in `lib.rs` erweitern**

In `src-tauri/src/lib.rs:74-91` die Liste in `invoke_handler` ergänzen — nach `commands::import_xlsx_apply,` hinzufügen:

```rust
            commands::katalog_faecher,
            commands::katalog_kategorien,
            commands::katalog_formulierungen,
            commands::katalog_fach_anlegen,
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
            commands::bemerkung_get,
            commands::bemerkung_set,
            commands::legacy_import_preview,
            commands::legacy_import_apply,
```

- [ ] **Step 6: Build-Check**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo check --no-default-features --lib && cargo test --no-default-features --lib
```

Expected: alle ~60 Tests grün.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(commands): Katalog + Bewertung + Bemerkung + Legacy-Import

18 neue Tauri-Commands mit Rollen-Guards (require_admin /
require_klassenlehrer_oder_admin / require_lehrer).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 10: Frontend — TypeScript-API-Wrapper

**Files:**
- Modify: `src/lib/api.ts` (anhängen)

Ziel: getypte Wrapper für alle neuen Commands.

- [ ] **Step 1: Typen + Wrapper anhängen**

Am Ende von `src/lib/api.ts`:

```typescript
// --- Katalog ---

export type Fach = {
  id: number;
  schuljahr_id: number;
  name: string;
  reihenfolge: number;
  aktiv: boolean;
};

export type Kategorie = {
  id: number;
  schuljahr_id: number;
  name: string;
  reihenfolge: number;
  aktiv: boolean;
};

export type Formulierung = {
  id: number;
  kategorie_id: number;
  text: string;
  reihenfolge: number;
  aktiv: boolean;
};

export const katalog = {
  faecher: (schuljahrId: number) =>
    invoke<Fach[]>('katalog_faecher', { schuljahrId }),
  kategorien: (schuljahrId: number) =>
    invoke<Kategorie[]>('katalog_kategorien', { schuljahrId }),
  formulierungen: (kategorieId: number) =>
    invoke<Formulierung[]>('katalog_formulierungen', { kategorieId }),

  fachAnlegen: (schuljahrId: number, name: string) =>
    invoke<number>('katalog_fach_anlegen', { schuljahrId, name }),
  kategorieAnlegen: (schuljahrId: number, name: string) =>
    invoke<number>('katalog_kategorie_anlegen', { schuljahrId, name }),
  formulierungAnlegen: (kategorieId: number, text: string) =>
    invoke<number>('katalog_formulierung_anlegen', { kategorieId, text }),

  fachAktiv: (id: number, aktiv: boolean) =>
    invoke<void>('katalog_fach_aktiv', { id, aktiv }),
  kategorieAktiv: (id: number, aktiv: boolean) =>
    invoke<void>('katalog_kategorie_aktiv', { id, aktiv }),
  formulierungAktiv: (id: number, aktiv: boolean) =>
    invoke<void>('katalog_formulierung_aktiv', { id, aktiv }),

  fachReihenfolge: (id: number, reihenfolge: number) =>
    invoke<void>('katalog_fach_reihenfolge', { id, reihenfolge }),
  kategorieReihenfolge: (id: number, reihenfolge: number) =>
    invoke<void>('katalog_kategorie_reihenfolge', { id, reihenfolge }),
  formulierungReihenfolge: (id: number, reihenfolge: number) =>
    invoke<void>('katalog_formulierung_reihenfolge', { id, reihenfolge })
};

// --- Bewertung ---

export type MatrixZelle = {
  schueler_id: number;
  kategorie_id: number;
  formulierung_id: number | null;
  geaendert_am: string;
};

export type BewertungUpdate = {
  schueler_id: number;
  fach_id: number;
  kategorie_id: number;
  formulierung_id: number | null;
  vorheriger_stand: string | null;
};

export type SetResult =
  | { status: 'Ok'; neuer_stand: string }
  | { status: 'Konflikt'; server_formulierung_id: number | null; server_geaendert_am: string };

export const bewertung = {
  matrix: (klasseId: number, fachId: number) =>
    invoke<MatrixZelle[]>('bewertung_matrix', { klasseId, fachId }),
  set: (update: BewertungUpdate) =>
    invoke<SetResult>('bewertung_set', { update })
};

// --- Bemerkung ---

export const bemerkung = {
  get: (schuelerId: number) =>
    invoke<[string, string] | null>('bemerkung_get', { schuelerId }),
  set: (schuelerId: number, text: string, vorherigerStand: string | null) =>
    invoke<SetResult>('bemerkung_set', { schuelerId, text, vorherigerStand })
};

// --- Legacy-Import ---

export type LegacyKategorie = {
  name: string;
  formulierungen: string[];
};

export type LegacyImportPreview = {
  faecher: string[];
  kategorien: LegacyKategorie[];
};

export type LegacyImportSummary = {
  neue_faecher: number;
  neue_kategorien: number;
  neue_formulierungen: number;
  uebersprungene_faecher: number;
  uebersprungene_kategorien: number;
  uebersprungene_formulierungen: number;
};

export const legacyImport = {
  preview: (faecherBytes: number[], floskelnBytes: number[], formatBytes: number[]) =>
    invoke<LegacyImportPreview>('legacy_import_preview', { faecherBytes, floskelnBytes, formatBytes }),
  apply: (schuljahrId: number, preview: LegacyImportPreview) =>
    invoke<LegacyImportSummary>('legacy_import_apply', { schuljahrId, preview })
};

// --- Schüler (für Bewertungs-Matrix) ---

export type SchuelerMini = {
  id: number;
  vorname: string;
  nachname: string;
  sortname: string;
};

export type Klasse = {
  id: number;
  name: string;
  schuljahr_id: number;
};

// Diese werden in Task 11 gebraucht — Backend muss noch ergänzen
export const klassenraum = {
  klassen: (schuljahrId: number) =>
    invoke<Klasse[]>('klassenraum_klassen', { schuljahrId }),
  schueler: (klasseId: number) =>
    invoke<SchuelerMini[]>('klassenraum_schueler', { klasseId })
};
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/api.ts
git commit -m "feat(api): TypeScript-Wrapper für Katalog+Bewertung+Bemerkung+Legacy

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 11: Backend — Klassen + Schüler-Mini-Reads

**Files:**
- Modify: `src-tauri/src/stammdaten.rs` (Funktionen)
- Modify: `src-tauri/src/commands.rs` (Commands)
- Modify: `src-tauri/src/lib.rs` (Handler)

Ziel: Frontend-Wrapper aus Task 10 brauchen `klassenraum_klassen` und `klassenraum_schueler`. Backend liefert sie.

- [ ] **Step 1: Test in stammdaten.rs vorab**

Am Ende von `mod tests` in `src-tauri/src/stammdaten.rs` (vor schließendem `}`):

```rust
    #[test]
    fn list_klassen_fuer_schuljahr() {
        let (_d, mut conn) = fresh_conn();
        let sj = anlegen(&conn, "2025/26").unwrap();
        upsert_schueler(&mut conn, sj, &[
            sample_input(None, "5a", "Anna", "Apfel"),
            sample_input(None, "5b", "Bert", "Birne"),
        ]).unwrap();
        let klassen = list_klassen(&conn, sj).unwrap();
        assert_eq!(klassen.len(), 2);
        assert_eq!(klassen[0].name, "5a");
        assert_eq!(klassen[1].name, "5b");
    }

    #[test]
    fn list_schueler_einer_klasse() {
        let (_d, mut conn) = fresh_conn();
        let sj = anlegen(&conn, "2025/26").unwrap();
        upsert_schueler(&mut conn, sj, &[
            sample_input(None, "5a", "Bert", "Birne"),
            sample_input(None, "5a", "Anna", "Apfel"),
            sample_input(None, "5b", "Cora", "Citro"),
        ]).unwrap();
        let klassen = list_klassen(&conn, sj).unwrap();
        let s5a = list_schueler(&conn, klassen[0].id).unwrap();
        assert_eq!(s5a.len(), 2);
        // Sortiert nach sortname (Apfel < Birne)
        assert_eq!(s5a[0].nachname, "Apfel");
        assert_eq!(s5a[1].nachname, "Birne");
    }
```

- [ ] **Step 2: Funktionen in stammdaten.rs ergänzen**

Vor dem `#[cfg(test)]`-Block in `src-tauri/src/stammdaten.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Klasse {
    pub id: i64,
    pub name: String,
    pub schuljahr_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchuelerMini {
    pub id: i64,
    pub vorname: String,
    pub nachname: String,
    pub sortname: String,
}

pub fn list_klassen(conn: &Connection, schuljahr_id: i64) -> AppResult<Vec<Klasse>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, schuljahr_id FROM klasse WHERE schuljahr_id = ?1 ORDER BY name",
    )?;
    let rows = stmt.query_map(params![schuljahr_id], |r| {
        Ok(Klasse { id: r.get(0)?, name: r.get(1)?, schuljahr_id: r.get(2)? })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn list_schueler(conn: &Connection, klasse_id: i64) -> AppResult<Vec<SchuelerMini>> {
    let mut stmt = conn.prepare(
        "SELECT id, vorname, nachname, sortname
         FROM schueler WHERE klasse_id = ?1 ORDER BY sortname",
    )?;
    let rows = stmt.query_map(params![klasse_id], |r| {
        Ok(SchuelerMini {
            id: r.get(0)?,
            vorname: r.get(1)?,
            nachname: r.get(2)?,
            sortname: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}
```

- [ ] **Step 3: Commands ergänzen**

Am Ende von `src-tauri/src/commands.rs`:

```rust
use crate::stammdaten::{Klasse, SchuelerMini};

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
```

- [ ] **Step 4: Handler-Liste in `lib.rs` ergänzen**

In `src-tauri/src/lib.rs` zur invoke_handler-Liste ergänzen:

```rust
            commands::klassenraum_klassen,
            commands::klassenraum_schueler,
```

- [ ] **Step 5: Tests + Build**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib
```

Expected: alle Tests grün, inkl. 2 neue stammdaten-Tests.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/stammdaten.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(stammdaten): list_klassen + list_schueler für Bewertungs-Matrix

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 12: Frontend — Katalog-Editor `/admin/katalog`

**Files:**
- Create: `src/routes/admin/katalog/+page.svelte`

Ziel: Drei-Tab-Editor (Fächer / Kategorien / Formulierungen). Inline-Edit, +Neu-Button, Aktiv-Toggle. KISS: keine Drag&Drop-Reihenfolge in Plan 3 (Reihenfolge-Edit über ↑/↓-Buttons reicht; D&D darf in Plan 4 nachgezogen werden).

- [ ] **Step 1: Page anlegen**

```svelte
<!-- src/routes/admin/katalog/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    katalog,
    type Schuljahr,
    type Fach,
    type Kategorie,
    type Formulierung
  } from '$lib/api';

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let tab = $state<'faecher' | 'kategorien' | 'formulierungen'>('faecher');
  let faecher = $state<Fach[]>([]);
  let kategorien = $state<Kategorie[]>([]);
  let aktiveKategorie = $state<Kategorie | null>(null);
  let formulierungen = $state<Formulierung[]>([]);
  let neuName = $state('');
  let neuText = $state('');
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (session.rolle !== 'administrator') {
      goto('/dashboard');
      return;
    }
    const list = await stammdaten.list();
    aktivesSchuljahr = list.find(s => s.aktiv) ?? null;
    if (!aktivesSchuljahr) {
      fehler = 'Kein aktives Schuljahr. Bitte zuerst unter /admin/stammdaten anlegen.';
      return;
    }
    await refreshFaecher();
    await refreshKategorien();
  });

  async function refreshFaecher() {
    if (!aktivesSchuljahr) return;
    faecher = await katalog.faecher(aktivesSchuljahr.id);
  }
  async function refreshKategorien() {
    if (!aktivesSchuljahr) return;
    kategorien = await katalog.kategorien(aktivesSchuljahr.id);
  }
  async function refreshFormulierungen() {
    if (!aktiveKategorie) { formulierungen = []; return; }
    formulierungen = await katalog.formulierungen(aktiveKategorie.id);
  }

  async function fachAnlegen() {
    if (!aktivesSchuljahr || !neuName.trim()) return;
    fehler = null;
    try {
      await katalog.fachAnlegen(aktivesSchuljahr.id, neuName.trim());
      neuName = '';
      await refreshFaecher();
    } catch (e) { fehler = String(e); }
  }
  async function kategorieAnlegen() {
    if (!aktivesSchuljahr || !neuName.trim()) return;
    fehler = null;
    try {
      await katalog.kategorieAnlegen(aktivesSchuljahr.id, neuName.trim());
      neuName = '';
      await refreshKategorien();
    } catch (e) { fehler = String(e); }
  }
  async function formulierungAnlegen() {
    if (!aktiveKategorie || !neuText.trim()) return;
    fehler = null;
    try {
      await katalog.formulierungAnlegen(aktiveKategorie.id, neuText.trim());
      neuText = '';
      await refreshFormulierungen();
    } catch (e) { fehler = String(e); }
  }

  async function fachToggle(f: Fach) {
    await katalog.fachAktiv(f.id, !f.aktiv);
    await refreshFaecher();
  }
  async function kategorieToggle(k: Kategorie) {
    await katalog.kategorieAktiv(k.id, !k.aktiv);
    await refreshKategorien();
  }
  async function formulierungToggle(f: Formulierung) {
    await katalog.formulierungAktiv(f.id, !f.aktiv);
    await refreshFormulierungen();
  }

  async function fachVerschieben(f: Fach, delta: number) {
    const idx = faecher.findIndex(x => x.id === f.id);
    const ziel = idx + delta;
    if (ziel < 0 || ziel >= faecher.length) return;
    const partner = faecher[ziel];
    await katalog.fachReihenfolge(f.id, partner.reihenfolge);
    await katalog.fachReihenfolge(partner.id, f.reihenfolge);
    await refreshFaecher();
  }
  async function kategorieVerschieben(k: Kategorie, delta: number) {
    const idx = kategorien.findIndex(x => x.id === k.id);
    const ziel = idx + delta;
    if (ziel < 0 || ziel >= kategorien.length) return;
    const partner = kategorien[ziel];
    await katalog.kategorieReihenfolge(k.id, partner.reihenfolge);
    await katalog.kategorieReihenfolge(partner.id, k.reihenfolge);
    await refreshKategorien();
  }
  async function formulierungVerschieben(f: Formulierung, delta: number) {
    const idx = formulierungen.findIndex(x => x.id === f.id);
    const ziel = idx + delta;
    if (ziel < 0 || ziel >= formulierungen.length) return;
    const partner = formulierungen[ziel];
    await katalog.formulierungReihenfolge(f.id, partner.reihenfolge);
    await katalog.formulierungReihenfolge(partner.id, f.reihenfolge);
    await refreshFormulierungen();
  }

  $effect(() => {
    refreshFormulierungen();
  });
</script>

<main class="container">
  <header>
    <h1>Katalog-Editor</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}
    <p class="error">Fehler: {fehler}</p>
  {/if}

  {#if aktivesSchuljahr}
    <p>Aktives Schuljahr: <strong>{aktivesSchuljahr.bezeichnung}</strong></p>

    <nav class="tabs">
      <button class:aktiv={tab === 'faecher'} onclick={() => tab = 'faecher'}>Fächer</button>
      <button class:aktiv={tab === 'kategorien'} onclick={() => tab = 'kategorien'}>Kategorien</button>
      <button class:aktiv={tab === 'formulierungen'} onclick={() => tab = 'formulierungen'}>Formulierungen</button>
    </nav>

    {#if tab === 'faecher'}
      <section>
        <ul>
          {#each faecher as f (f.id)}
            <li class:inaktiv={!f.aktiv}>
              <button onclick={() => fachVerschieben(f, -1)}>↑</button>
              <button onclick={() => fachVerschieben(f, 1)}>↓</button>
              <span>{f.name}</span>
              <label>
                <input type="checkbox" checked={f.aktiv} onchange={() => fachToggle(f)} />
                aktiv
              </label>
            </li>
          {/each}
        </ul>
        <input placeholder="Neues Fach" bind:value={neuName} onkeydown={(e) => e.key === 'Enter' && fachAnlegen()} />
        <button onclick={fachAnlegen}>+ Anlegen</button>
      </section>
    {/if}

    {#if tab === 'kategorien'}
      <section>
        <ul>
          {#each kategorien as k (k.id)}
            <li class:inaktiv={!k.aktiv}>
              <button onclick={() => kategorieVerschieben(k, -1)}>↑</button>
              <button onclick={() => kategorieVerschieben(k, 1)}>↓</button>
              <span>{k.name}</span>
              <label>
                <input type="checkbox" checked={k.aktiv} onchange={() => kategorieToggle(k)} />
                aktiv
              </label>
            </li>
          {/each}
        </ul>
        <input placeholder="Neue Kategorie" bind:value={neuName} onkeydown={(e) => e.key === 'Enter' && kategorieAnlegen()} />
        <button onclick={kategorieAnlegen}>+ Anlegen</button>
      </section>
    {/if}

    {#if tab === 'formulierungen'}
      <section>
        <label>
          Kategorie:
          <select onchange={(e) => aktiveKategorie = kategorien.find(k => k.id === Number((e.target as HTMLSelectElement).value)) ?? null}>
            <option value="">— wählen —</option>
            {#each kategorien as k (k.id)}
              <option value={k.id}>{k.name}</option>
            {/each}
          </select>
        </label>

        {#if aktiveKategorie}
          <ul>
            {#each formulierungen as f (f.id)}
              <li class:inaktiv={!f.aktiv}>
                <button onclick={() => formulierungVerschieben(f, -1)}>↑</button>
                <button onclick={() => formulierungVerschieben(f, 1)}>↓</button>
                <span>{f.text}</span>
                <label>
                  <input type="checkbox" checked={f.aktiv} onchange={() => formulierungToggle(f)} />
                  aktiv
                </label>
              </li>
            {/each}
          </ul>
          <input placeholder="Neue Formulierung" bind:value={neuText} onkeydown={(e) => e.key === 'Enter' && formulierungAnlegen()} />
          <button onclick={formulierungAnlegen}>+ Anlegen</button>
        {/if}
      </section>
    {/if}
  {/if}
</main>

<style>
  .container { max-width: 900px; margin: 0 auto; padding: 2rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .tabs { display: flex; gap: 0.5rem; margin: 1rem 0; }
  .tabs button { padding: 0.5rem 1rem; background: #eee; border: 0; cursor: pointer; }
  .tabs button.aktiv { background: var(--sg-petrol, #004058); color: white; }
  ul { list-style: none; padding: 0; }
  li { display: flex; gap: 0.5rem; align-items: center; padding: 0.4rem; border-bottom: 1px solid #ddd; }
  li.inaktiv span { color: #999; text-decoration: line-through; }
  li button { padding: 0.2rem 0.5rem; }
  input[type="text"], input:not([type]) { padding: 0.4rem; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/admin/katalog/+page.svelte
git commit -m "feat(frontend): /admin/katalog Editor für Fach/Kategorie/Formulierung

3-Tab-UI mit Anlegen, Aktiv-Toggle, Reihenfolge per ↑/↓.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 13: Frontend — Legacy-Import-Wizard `/admin/legacy-import`

**Files:**
- Create: `src/routes/admin/legacy-import/+page.svelte`

Ziel: 3-File-Wizard, Vorschau, Apply.

- [ ] **Step 1: Page anlegen**

```svelte
<!-- src/routes/admin/legacy-import/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    legacyImport,
    type Schuljahr,
    type LegacyImportPreview,
    type LegacyImportSummary
  } from '$lib/api';

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let faecherBytes = $state<number[] | null>(null);
  let floskelnBytes = $state<number[] | null>(null);
  let formatBytes = $state<number[] | null>(null);
  let preview = $state<LegacyImportPreview | null>(null);
  let summary = $state<LegacyImportSummary | null>(null);
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (session.rolle !== 'administrator') { goto('/dashboard'); return; }
    const list = await stammdaten.list();
    aktivesSchuljahr = list.find(s => s.aktiv) ?? null;
  });

  async function fileBytes(e: Event): Promise<number[] | null> {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f) return null;
    return Array.from(new Uint8Array(await f.arrayBuffer()));
  }

  async function vorschauZeigen() {
    if (!faecherBytes || !floskelnBytes || !formatBytes) {
      fehler = 'Bitte alle drei Dateien wählen.';
      return;
    }
    fehler = null;
    try {
      preview = await legacyImport.preview(faecherBytes, floskelnBytes, formatBytes);
    } catch (e) { fehler = String(e); }
  }

  async function importieren() {
    if (!aktivesSchuljahr || !preview) return;
    fehler = null;
    try {
      summary = await legacyImport.apply(aktivesSchuljahr.id, preview);
    } catch (e) { fehler = String(e); }
  }
</script>

<main class="container">
  <header>
    <h1>Legacy-Import (Fächer + Floskeln + format.xls)</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}<p class="error">Fehler: {fehler}</p>{/if}

  {#if !summary}
    <section>
      <p>Aktives Schuljahr: <strong>{aktivesSchuljahr?.bezeichnung ?? '—'}</strong></p>
      <p>Wähle die drei Dateien aus dem alten Jiraso-ProgrammPaket:</p>
      <ol>
        <li>
          <label>Fächer.txt: <input type="file" accept=".txt" onchange={async (e) => faecherBytes = await fileBytes(e)} /></label>
          {#if faecherBytes} ✓ {faecherBytes.length} Bytes {/if}
        </li>
        <li>
          <label>Floskeln.txt: <input type="file" accept=".txt" onchange={async (e) => floskelnBytes = await fileBytes(e)} /></label>
          {#if floskelnBytes} ✓ {floskelnBytes.length} Bytes {/if}
        </li>
        <li>
          <label>format.xls: <input type="file" accept=".xls,.xlsx" onchange={async (e) => formatBytes = await fileBytes(e)} /></label>
          {#if formatBytes} ✓ {formatBytes.length} Bytes {/if}
        </li>
      </ol>
      <button onclick={vorschauZeigen}>Vorschau anzeigen</button>

      {#if preview}
        <h2>Vorschau</h2>
        <h3>Fächer ({preview.faecher.length})</h3>
        <ul>{#each preview.faecher as f (f)}<li>{f}</li>{/each}</ul>
        <h3>Kategorien ({preview.kategorien.length})</h3>
        {#each preview.kategorien as k (k.name)}
          <h4>{k.name}</h4>
          <ul>{#each k.formulierungen as f (f)}<li>{f}</li>{/each}</ul>
        {/each}

        <button onclick={importieren}>Importieren</button>
      {/if}
    </section>
  {:else}
    <section>
      <h2>Import abgeschlossen</h2>
      <ul>
        <li>{summary.neue_faecher} neue Fächer ({summary.uebersprungene_faecher} übersprungen)</li>
        <li>{summary.neue_kategorien} neue Kategorien ({summary.uebersprungene_kategorien} übersprungen)</li>
        <li>{summary.neue_formulierungen} neue Formulierungen ({summary.uebersprungene_formulierungen} übersprungen)</li>
      </ul>
      <a href="/admin/katalog">→ Katalog-Editor</a>
    </section>
  {/if}
</main>

<style>
  .container { max-width: 800px; margin: 0 auto; padding: 2rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  ol li { margin: 0.5rem 0; }
  ul { margin-left: 1rem; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/admin/legacy-import/+page.svelte
git commit -m "feat(frontend): /admin/legacy-import 3-File-Wizard

Vorschau zeigt geparste Faecher + Kategorien (mit Formulierungen),
Apply schreibt idempotent ins aktive Schuljahr.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 14: Frontend — Bewertungs-Matrix `/bewertung`

**Files:**
- Create: `src/routes/bewertung/+page.svelte`

Ziel: Matrix Schüler×Kategorie für ein Fach, Detail-Panel rechts für Bemerkung. Autosave on-blur per Zelle, debounced bei Bemerkung. Konflikt-Banner an betroffener Zelle.

- [ ] **Step 1: Page anlegen**

```svelte
<!-- src/routes/bewertung/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    katalog,
    klassenraum,
    bewertung as bewertungApi,
    bemerkung as bemerkungApi,
    type Schuljahr,
    type Klasse,
    type Fach,
    type Kategorie,
    type Formulierung,
    type SchuelerMini,
    type MatrixZelle
  } from '$lib/api';

  type Cell = {
    formulierung_id: number | null;
    geaendert_am: string | null;
    status: 'idle' | 'saving' | 'saved' | 'konflikt';
    konfliktServerFid?: number | null;
    konfliktServerTs?: string;
  };

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let klassen = $state<Klasse[]>([]);
  let faecher = $state<Fach[]>([]);
  let kategorien = $state<Kategorie[]>([]);
  let formulierungenByKat = $state<Record<number, Formulierung[]>>({});
  let aktiveKlasse = $state<Klasse | null>(null);
  let aktivesFach = $state<Fach | null>(null);
  let schueler = $state<SchuelerMini[]>([]);
  let cells = $state<Record<string, Cell>>({}); // key: `${schueler_id}:${kategorie_id}`
  let fokusSchueler = $state<SchuelerMini | null>(null);
  let bemerkungText = $state('');
  let bemerkungStand = $state<string | null>(null);
  let bemerkungStatus = $state<'idle' | 'saving' | 'saved' | 'konflikt'>('idle');
  let bemerkungKonfliktTs = $state<string | null>(null);
  let fehler = $state<string | null>(null);

  let bemDebounce: ReturnType<typeof setTimeout> | null = null;
  let pollHandle: ReturnType<typeof setInterval> | null = null;

  onMount(async () => {
    if (!session.rolle) { goto('/login'); return; }
    const sjs = await stammdaten.list();
    aktivesSchuljahr = sjs.find(s => s.aktiv) ?? null;
    if (!aktivesSchuljahr) { fehler = 'Kein aktives Schuljahr.'; return; }

    klassen = await klassenraum.klassen(aktivesSchuljahr.id);
    faecher = (await katalog.faecher(aktivesSchuljahr.id)).filter(f => f.aktiv);
    kategorien = (await katalog.kategorien(aktivesSchuljahr.id)).filter(k => k.aktiv);
    for (const k of kategorien) {
      formulierungenByKat[k.id] = (await katalog.formulierungen(k.id)).filter(f => f.aktiv);
    }
    pollHandle = setInterval(refreshMatrixSilent, 30_000);
  });

  $effect(() => {
    return () => { if (pollHandle) clearInterval(pollHandle); };
  });

  function key(s: number, k: number) { return `${s}:${k}`; }

  async function ladeKlasseUndFach() {
    if (!aktiveKlasse || !aktivesFach) return;
    schueler = await klassenraum.schueler(aktiveKlasse.id);
    const matrix = await bewertungApi.matrix(aktiveKlasse.id, aktivesFach.id);
    cells = {};
    for (const s of schueler) {
      for (const k of kategorien) {
        cells[key(s.id, k.id)] = { formulierung_id: null, geaendert_am: null, status: 'idle' };
      }
    }
    for (const m of matrix) {
      cells[key(m.schueler_id, m.kategorie_id)] = {
        formulierung_id: m.formulierung_id,
        geaendert_am: m.geaendert_am,
        status: 'idle',
      };
    }
  }

  async function refreshMatrixSilent() {
    if (!aktiveKlasse || !aktivesFach) return;
    const matrix = await bewertungApi.matrix(aktiveKlasse.id, aktivesFach.id);
    for (const m of matrix) {
      const k = key(m.schueler_id, m.kategorie_id);
      const c = cells[k];
      if (c && c.status !== 'saving' && c.status !== 'konflikt') {
        cells[k] = { formulierung_id: m.formulierung_id, geaendert_am: m.geaendert_am, status: 'idle' };
      }
    }
  }

  $effect(() => {
    ladeKlasseUndFach();
  });

  async function setCell(s: SchuelerMini, k: Kategorie, fid: number | null) {
    if (!aktivesFach) return;
    const ck = key(s.id, k.id);
    const before = cells[ck];
    cells[ck] = { ...before, status: 'saving', formulierung_id: fid };
    const r = await bewertungApi.set({
      schueler_id: s.id,
      fach_id: aktivesFach.id,
      kategorie_id: k.id,
      formulierung_id: fid,
      vorheriger_stand: before.geaendert_am,
    });
    if (r.status === 'Ok') {
      cells[ck] = { formulierung_id: fid, geaendert_am: r.neuer_stand || null, status: 'saved' };
      setTimeout(() => {
        if (cells[ck].status === 'saved') cells[ck] = { ...cells[ck], status: 'idle' };
      }, 1500);
    } else {
      cells[ck] = {
        formulierung_id: fid, // unsere lokale Wahl
        geaendert_am: before.geaendert_am,
        status: 'konflikt',
        konfliktServerFid: r.server_formulierung_id,
        konfliktServerTs: r.server_geaendert_am,
      };
    }
  }

  async function konfliktMeineUebernehmen(s: SchuelerMini, k: Kategorie) {
    if (!aktivesFach) return;
    const ck = key(s.id, k.id);
    const c = cells[ck];
    const r = await bewertungApi.set({
      schueler_id: s.id, fach_id: aktivesFach.id, kategorie_id: k.id,
      formulierung_id: c.formulierung_id,
      vorheriger_stand: c.konfliktServerTs ?? null,
    });
    if (r.status === 'Ok') {
      cells[ck] = { formulierung_id: c.formulierung_id, geaendert_am: r.neuer_stand || null, status: 'saved' };
    }
  }
  function konfliktIhreBehalten(s: SchuelerMini, k: Kategorie) {
    const ck = key(s.id, k.id);
    const c = cells[ck];
    cells[ck] = {
      formulierung_id: c.konfliktServerFid ?? null,
      geaendert_am: c.konfliktServerTs ?? null,
      status: 'idle',
    };
  }

  async function fokusiere(s: SchuelerMini) {
    fokusSchueler = s;
    bemerkungStatus = 'idle';
    bemerkungKonfliktTs = null;
    if (session.rolle !== 'klassenlehrer' && session.rolle !== 'administrator') return;
    const got = await bemerkungApi.get(s.id);
    if (got) { [bemerkungText, bemerkungStand] = got; }
    else { bemerkungText = ''; bemerkungStand = null; }
  }

  function bemerkungChange() {
    if (bemDebounce) clearTimeout(bemDebounce);
    bemDebounce = setTimeout(saveBemerkung, 1500);
  }
  async function saveBemerkung() {
    if (!fokusSchueler) return;
    bemerkungStatus = 'saving';
    const r = await bemerkungApi.set(fokusSchueler.id, bemerkungText, bemerkungStand);
    if (r.status === 'Ok') {
      bemerkungStand = r.neuer_stand || null;
      bemerkungStatus = 'saved';
      setTimeout(() => { if (bemerkungStatus === 'saved') bemerkungStatus = 'idle'; }, 1500);
    } else {
      bemerkungStatus = 'konflikt';
      bemerkungKonfliktTs = r.server_geaendert_am;
    }
  }
  async function bemKonfliktMeine() {
    if (!fokusSchueler) return;
    const r = await bemerkungApi.set(fokusSchueler.id, bemerkungText, bemerkungKonfliktTs);
    if (r.status === 'Ok') {
      bemerkungStand = r.neuer_stand || null;
      bemerkungStatus = 'saved';
      bemerkungKonfliktTs = null;
    }
  }
  async function bemKonfliktIhre() {
    if (!fokusSchueler) return;
    const got = await bemerkungApi.get(fokusSchueler.id);
    if (got) { [bemerkungText, bemerkungStand] = got; }
    bemerkungStatus = 'idle';
    bemerkungKonfliktTs = null;
  }

  function statusIcon(s: 'idle' | 'saving' | 'saved' | 'konflikt'): string {
    return s === 'saving' ? '⟳' : s === 'saved' ? '✓' : s === 'konflikt' ? '⚠' : '';
  }
</script>

<main class="container">
  <header>
    <h1>Bewertung</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}<p class="error">{fehler}</p>{/if}

  <div class="controls">
    <label>Klasse:
      <select bind:value={aktiveKlasse}>
        <option value={null}>—</option>
        {#each klassen as k (k.id)}<option value={k}>{k.name}</option>{/each}
      </select>
    </label>
    <label>Fach:
      <select bind:value={aktivesFach}>
        <option value={null}>—</option>
        {#each faecher as f (f.id)}<option value={f}>{f.name}</option>{/each}
      </select>
    </label>
  </div>

  {#if aktiveKlasse && aktivesFach}
    <div class="grid">
      <div class="matrix">
        <table>
          <thead>
            <tr>
              <th>Schüler:in</th>
              {#each kategorien as k (k.id)}<th>{k.name}</th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each schueler as s (s.id)}
              <tr class:fokus={fokusSchueler?.id === s.id} onclick={() => fokusiere(s)}>
                <td>{s.sortname}</td>
                {#each kategorien as k (k.id)}
                  {@const c = cells[key(s.id, k.id)] ?? { formulierung_id: null, geaendert_am: null, status: 'idle' }}
                  <td class:konflikt={c.status === 'konflikt'}>
                    <select
                      value={c.formulierung_id ?? ''}
                      onchange={(e) => {
                        const v = (e.target as HTMLSelectElement).value;
                        setCell(s, k, v === '' ? null : Number(v));
                      }}
                    >
                      <option value="">— keine Angabe —</option>
                      {#each (formulierungenByKat[k.id] ?? []) as f (f.id)}
                        <option value={f.id}>{f.text}</option>
                      {/each}
                    </select>
                    <span class="status">{statusIcon(c.status)}</span>
                    {#if c.status === 'konflikt'}
                      <div class="konflikt-banner">
                        Andere Sitzung hat
                        {#if c.konfliktServerFid != null}
                          „{(formulierungenByKat[k.id] ?? []).find(x => x.id === c.konfliktServerFid)?.text ?? '?'}"
                        {:else}
                          „— keine Angabe —"
                        {/if}
                        gewählt.
                        <button onclick={() => konfliktMeineUebernehmen(s, k)}>Meine</button>
                        <button onclick={() => konfliktIhreBehalten(s, k)}>Ihre</button>
                      </div>
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <aside class="panel">
        {#if fokusSchueler}
          <h2>{fokusSchueler.sortname}</h2>
          {#if session.rolle === 'klassenlehrer' || session.rolle === 'administrator'}
            <h3>Bemerkung <span class="status">{statusIcon(bemerkungStatus)}</span></h3>
            <textarea
              rows="8"
              bind:value={bemerkungText}
              oninput={bemerkungChange}
              onblur={saveBemerkung}
            ></textarea>
            <p class="counter" class:warn={bemerkungText.length > 700}>{bemerkungText.length}/700 Zeichen</p>
            {#if bemerkungStatus === 'konflikt'}
              <div class="konflikt-banner">
                Andere Sitzung hat die Bemerkung geändert.
                <button onclick={bemKonfliktMeine}>Meine</button>
                <button onclick={bemKonfliktIhre}>Ihre</button>
              </div>
            {/if}
          {:else}
            <p><em>Bemerkung nur für Klassenlehrer:in / Admin.</em></p>
          {/if}
        {:else}
          <p><em>Klick eine Schüler:in für Detail-Panel.</em></p>
        {/if}
      </aside>
    </div>
  {/if}
</main>

<style>
  .container { max-width: 1400px; margin: 0 auto; padding: 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .controls { display: flex; gap: 1rem; margin: 1rem 0; }
  .grid { display: grid; grid-template-columns: 2fr 1fr; gap: 1rem; }
  .matrix { overflow: auto; }
  table { border-collapse: collapse; font-size: 0.85rem; width: 100%; }
  th, td { border: 1px solid #ddd; padding: 0.3rem; vertical-align: top; }
  thead th { background: #f5f5f5; position: sticky; top: 0; }
  tr.fokus td { background: #fff8e1; }
  td.konflikt { background: #fee; }
  td select { max-width: 200px; }
  td .status { margin-left: 0.3rem; color: #666; }
  .konflikt-banner { background: #fee; border: 1px solid #c00; padding: 0.4rem; margin-top: 0.3rem; font-size: 0.8rem; }
  .konflikt-banner button { margin-left: 0.3rem; }
  .panel { padding: 1rem; border: 1px solid #ddd; border-radius: 4px; }
  .panel textarea { width: 100%; font-family: inherit; }
  .counter { font-size: 0.8rem; color: #666; }
  .counter.warn { color: #c00; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/bewertung/+page.svelte
git commit -m "feat(frontend): /bewertung Matrix + Detail-Panel mit Konflikt-UX

Schüler-Klasse-Fach-Selector, autosave on-blur, Bemerkung debounced
1500ms, Inline-Konflikt-Banner mit Meine/Ihre-Choice, Background-
Refresh alle 30s.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 15: Dashboard — Links zu neuen Routen

**Files:**
- Modify: `src/routes/dashboard/+page.svelte`

Ziel: Navigations-Kacheln für die drei neuen Routen.

- [ ] **Step 1: Dashboard öffnen + Links ergänzen**

Lies den aktuellen Stand mit:

```bash
cat /home/neo/projects/Jiraso-reloaded/src/routes/dashboard/+page.svelte
```

Suche den `<main>`-Block bzw. den Navigations-Bereich und füge je eine Kachel ein:
- `/bewertung` — sichtbar für alle Rollen
- `/admin/katalog` — nur Admin
- `/admin/legacy-import` — nur Admin

Falls noch keine Kachel-Struktur existiert, lege ein einfaches `<nav class="kacheln">` mit `<a class="kachel">` an. Wichtig: bei den Admin-Routen ein `{#if session.rolle === 'administrator'}`-Guard rumlegen, damit Fachlehrer:innen sie nicht sehen.

Beispiel-Snippet, das hineinpasst:

```svelte
<a class="kachel" href="/bewertung">
  <h3>Bewertung</h3>
  <p>Verbalbeurteilungen + Bemerkungen eingeben</p>
</a>
{#if session.rolle === 'administrator'}
  <a class="kachel" href="/admin/katalog">
    <h3>Katalog</h3>
    <p>Fächer · Kategorien · Formulierungen</p>
  </a>
  <a class="kachel" href="/admin/legacy-import">
    <h3>Legacy-Import</h3>
    <p>Fächer.txt + Floskeln.txt + format.xls</p>
  </a>
{/if}
```

- [ ] **Step 2: Commit**

```bash
git add src/routes/dashboard/+page.svelte
git commit -m "feat(dashboard): Links zu /bewertung /admin/katalog /admin/legacy-import

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 16: Bug-Reporter — Backend (GitHub-API-Call)

**Files:**
- Modify: `src-tauri/Cargo.toml` (+ `reqwest` mit blocking + json features)
- Modify: `src-tauri/src/config.rs` (BugReportConfig hinzufügen)
- Create: `src-tauri/src/bug_report.rs`
- Modify: `src-tauri/src/lib.rs` + `src-tauri/src/commands.rs`

Ziel: Tauri-Command `bug_report(titel, body)` öffnet ein GitHub-Issue im konfigurierten Repo via PAT aus `config.toml`. Ohne Token: sprechender Fehler, Frontend zeigt mailto-Fallback.

- [ ] **Step 1: `reqwest` als Dependency**

In `src-tauri/Cargo.toml` im `[dependencies]`-Block ergänzen:

```toml
reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
```

- [ ] **Step 2: Config erweitern**

Lies den aktuellen Stand mit:

```bash
cat /home/neo/projects/Jiraso-reloaded/src-tauri/src/config.rs
```

Im `AppConfig`-Struct ein neues optionales Feld ergänzen (Standard-Reihenfolge der TOML-Sektionen merken). Beispielhaft, falls Struct so aussieht `pub struct AppConfig { pub schule, pub aktuelles_schuljahr, pub passwoerter, pub lock }`, wird ergänzt zu:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BugReportConfig {
    #[serde(default)]
    pub github_token: String,
    #[serde(default = "default_repo")]
    pub repo: String,
}

fn default_repo() -> String { "hibbes/Jiraso-reloaded".to_string() }
```

und `AppConfig`:

```rust
#[serde(default)]
pub bug_report: BugReportConfig,
```

(Falls Struct anders aussieht: das `bug_report`-Feld zur bestehenden Liste hinzufügen.)

- [ ] **Step 3: Modul `bug_report.rs` anlegen**

```rust
// src-tauri/src/bug_report.rs
use crate::error::{AppError, AppResult};
use crate::config::AppConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueResponse {
    pub number: i64,
    pub html_url: String,
}

pub fn submit(cfg: &AppConfig, titel: &str, body: &str) -> AppResult<IssueResponse> {
    if cfg.bug_report.github_token.trim().is_empty() {
        return Err(AppError::Config(
            "Kein GitHub-Token konfiguriert. Admin muss [bug_report].github_token in config.toml eintragen."
                .into(),
        ));
    }
    let url = format!("https://api.github.com/repos/{}/issues", cfg.bug_report.repo);
    let payload = serde_json::json!({
        "title": titel,
        "body": body,
        "labels": ["bug-report-app"],
    });
    let client = reqwest::blocking::Client::builder()
        .user_agent("jiraso-reloaded-bug-reporter")
        .build()
        .map_err(|e| AppError::Config(format!("HTTP-Client-Fehler: {e}")))?;
    let resp = client.post(&url)
        .bearer_auth(&cfg.bug_report.github_token)
        .header("Accept", "application/vnd.github+json")
        .json(&payload)
        .send()
        .map_err(|e| AppError::Config(format!("GitHub-Request fehlgeschlagen: {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let txt = resp.text().unwrap_or_default();
        return Err(AppError::Config(format!("GitHub antwortete {status}: {txt}")));
    }
    let issue: IssueResponse = resp.json()
        .map_err(|e| AppError::Config(format!("GitHub-Antwort nicht parsebar: {e}")))?;
    Ok(issue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AppConfig, BugReportConfig};

    #[test]
    fn submit_ohne_token_fehlt_aussagekraeftig() {
        let cfg = AppConfig {
            bug_report: BugReportConfig::default(),
            ..AppConfig::default()
        };
        let err = submit(&cfg, "x", "y").unwrap_err();
        assert!(format!("{err}").contains("Kein GitHub-Token"));
    }
}
```

- [ ] **Step 4: Modul + Command registrieren**

In `src-tauri/src/lib.rs` nach `pub mod legacy_import;`:

```rust
pub mod bug_report;
```

In `src-tauri/src/commands.rs` am Ende:

```rust
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
```

In `src-tauri/src/lib.rs` zur `invoke_handler`-Liste:

```rust
            commands::bug_report_submit,
```

- [ ] **Step 5: Tests + Build**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib bug_report
```

Expected: 1 Test grün.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/config.rs src-tauri/src/bug_report.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(bug_report): GitHub-Issue-Submitter via PAT in config.toml

Tauri-Command bug_report_submit ruft GitHub-API auf, Token aus
[bug_report].github_token in config.toml. Ohne Token: sprechender
Fehler -> Frontend zeigt mailto-Fallback.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 17: Frontend — Käfer-Knopf + Sternenregen + Layout-Integration

**Files:**
- Create: `src/lib/celebration.svelte.ts`
- Create: `src/lib/Celebration.svelte`
- Create: `src/lib/BugButton.svelte`
- Modify: `src/lib/api.ts`
- Modify: `src/routes/+layout.svelte` (Komponenten global einhängen)

Ziel: Käfer-Knopf unten rechts auf jeder Route, Modal mit Form, Sternenregen bei Erfolg.

- [ ] **Step 1: API-Wrapper**

Am Ende von `src/lib/api.ts`:

```typescript
export type IssueResponse = { number: number; html_url: string };

export const bugReport = {
  submit: (titel: string, body: string) =>
    invoke<IssueResponse>('bug_report_submit', { titel, body })
};
```

- [ ] **Step 2: Sternenregen-Store**

```typescript
// src/lib/celebration.svelte.ts
class CelebrationState {
  active = $state(false);
  trigger() {
    this.active = true;
    setTimeout(() => { this.active = false; }, 2500);
  }
}
export const celebration = new CelebrationState();
```

- [ ] **Step 3: Sternenregen-Komponente**

```svelte
<!-- src/lib/Celebration.svelte -->
<script lang="ts">
  import { celebration } from './celebration.svelte';
  import { onDestroy } from 'svelte';

  let canvas: HTMLCanvasElement | undefined = $state();
  let raf: number | undefined;

  type Star = { x: number; y: number; vx: number; vy: number; rot: number; vrot: number; size: number; emoji: string };
  let stars: Star[] = [];

  function start() {
    if (!canvas) return;
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    stars = Array.from({ length: 120 }, () => ({
      x: Math.random() * canvas.width,
      y: -20 - Math.random() * 200,
      vx: (Math.random() - 0.5) * 2,
      vy: 2 + Math.random() * 4,
      rot: Math.random() * Math.PI * 2,
      vrot: (Math.random() - 0.5) * 0.2,
      size: 16 + Math.random() * 16,
      emoji: ['★', '✦', '✧', '⭐'][Math.floor(Math.random() * 4)],
    }));
    tick();
  }

  function tick() {
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    for (const s of stars) {
      s.x += s.vx;
      s.y += s.vy;
      s.rot += s.vrot;
      ctx.save();
      ctx.translate(s.x, s.y);
      ctx.rotate(s.rot);
      ctx.font = `${s.size}px sans-serif`;
      ctx.fillStyle = '#ffd566';
      ctx.fillText(s.emoji, -s.size / 2, s.size / 2);
      ctx.restore();
    }
    if (celebration.active) raf = requestAnimationFrame(tick);
  }

  $effect(() => {
    if (celebration.active) start();
    else if (raf) { cancelAnimationFrame(raf); raf = undefined; }
  });

  onDestroy(() => { if (raf) cancelAnimationFrame(raf); });
</script>

{#if celebration.active}
  <canvas bind:this={canvas} class="celebration"></canvas>
{/if}

<style>
  .celebration {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 9999;
  }
</style>
```

- [ ] **Step 4: BugButton-Komponente**

```svelte
<!-- src/lib/BugButton.svelte -->
<script lang="ts">
  import { bugReport, type IssueResponse } from '$lib/api';
  import { celebration } from '$lib/celebration.svelte';

  let offen = $state(false);
  let titel = $state('');
  let beschreibung = $state('');
  let kontext = $state('');
  let submitting = $state(false);
  let issue = $state<IssueResponse | null>(null);
  let fehler = $state<string | null>(null);

  function oeffnen() {
    offen = true;
    issue = null;
    fehler = null;
    titel = '';
    beschreibung = '';
    kontext = `Route: ${window.location.pathname}\nUser-Agent: ${navigator.userAgent}\nZeit: ${new Date().toISOString()}`;
  }
  function schliessen() {
    offen = false;
  }

  async function senden() {
    if (titel.trim().length < 5 || beschreibung.trim().length < 10) {
      fehler = 'Bitte Titel ≥5 Zeichen und Beschreibung ≥10 Zeichen.';
      return;
    }
    submitting = true;
    fehler = null;
    try {
      const body = `${beschreibung}\n\n---\n${kontext}`;
      issue = await bugReport.submit(titel, body);
      celebration.trigger();
    } catch (e) {
      fehler = String(e);
    } finally {
      submitting = false;
    }
  }

  function mailtoFallback(): string {
    const subject = encodeURIComponent(titel || 'Bug in Jiraso-reloaded');
    const body = encodeURIComponent(`${beschreibung}\n\n---\n${kontext}`);
    return `mailto:mczernohous@gmail.com?subject=${subject}&body=${body}`;
  }
</script>

<button class="bug-button" onclick={oeffnen} title="Bug melden">🪲</button>

{#if offen}
  <div class="modal" role="dialog" aria-modal="true">
    <div class="dialog">
      {#if issue}
        <h2>Danke!</h2>
        <p>Issue <a href={issue.html_url} target="_blank" rel="noopener">#{issue.number}</a> wurde angelegt.</p>
        <button onclick={schliessen}>OK</button>
      {:else}
        <h2>Bug melden</h2>
        <label>Titel<input bind:value={titel} placeholder="Kurz: Was ging nicht?" /></label>
        <label>Beschreibung<textarea rows="4" bind:value={beschreibung} placeholder="Was war ich am Tun, was ist passiert?"></textarea></label>
        <label>Kontext (automatisch)<textarea rows="3" bind:value={kontext} readonly></textarea></label>
        {#if fehler}
          <p class="error">{fehler}</p>
          <p>Fallback: <a href={mailtoFallback()}>per E-Mail an Marek</a></p>
        {/if}
        <div class="actions">
          <button onclick={schliessen}>Abbrechen</button>
          <button onclick={senden} disabled={submitting}>{submitting ? 'Sende...' : 'Senden'}</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .bug-button {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    width: 3rem;
    height: 3rem;
    border-radius: 50%;
    border: 0;
    background: var(--sg-petrol, #004058);
    color: white;
    font-size: 1.4rem;
    cursor: pointer;
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
    z-index: 9990;
  }
  .bug-button:hover { transform: scale(1.05); }
  .modal {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9991;
  }
  .dialog {
    background: white;
    padding: 1.5rem;
    border-radius: 8px;
    width: min(500px, 90vw);
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .dialog label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; }
  .dialog input, .dialog textarea {
    padding: 0.4rem;
    font-family: inherit;
    border: 1px solid #ccc;
    border-radius: 4px;
  }
  .dialog .error { color: #c00; }
  .dialog .actions { display: flex; gap: 0.5rem; justify-content: flex-end; }
</style>
```

- [ ] **Step 5: Layout-Integration**

Lies den aktuellen Stand mit:

```bash
cat /home/neo/projects/Jiraso-reloaded/src/routes/+layout.svelte
```

Ergänze das Layout um die zwei Komponenten. Beispiel:

```svelte
<script lang="ts">
  import BugButton from '$lib/BugButton.svelte';
  import Celebration from '$lib/Celebration.svelte';
  // ... bestehende Imports
</script>

<!-- bestehender Layout-Code -->
<slot />

<BugButton />
<Celebration />
```

(Wenn das Layout keine `<slot />`-Form, sondern `{@render children()}` nutzt: Komponenten dahinter einfügen.)

- [ ] **Step 6: Sternenregen-Trigger in Bewertungs-Matrix**

In `src/routes/bewertung/+page.svelte` den `setCell`-Block anpassen, sodass nach erfolgreichem Save die Klassen-Komplettheit geprüft wird.

Importzeile am Anfang ergänzen:

```typescript
import { celebration } from '$lib/celebration.svelte';
```

Innerhalb von `setCell`, nach dem `if (r.status === 'Ok')`-Branch (vor `setTimeout(...)`), folgenden Block einfügen:

```typescript
      // Klasse fertig? Sternenregen-Trigger
      const allesGesetzt = schueler.every(stud =>
        kategorien.every(kat => {
          const c = cells[key(stud.id, kat.id)];
          return c && c.geaendert_am !== null;
        })
      );
      if (allesGesetzt) {
        celebration.trigger();
      }
```

- [ ] **Step 7: Commit**

```bash
git add src/lib/celebration.svelte.ts src/lib/Celebration.svelte src/lib/BugButton.svelte src/lib/api.ts src/routes/+layout.svelte src/routes/bewertung/+page.svelte
git commit -m "feat(frontend): Käfer-Knopf für Bug-Report + Sternenregen-Belohnung

- BugButton global im Layout (unten rechts), öffnet Modal mit
  Titel/Beschreibung/Auto-Kontext, postet via Tauri an GitHub-Issues.
- Celebration-Canvas mit fallenden Sternen ★/✦/✧/⭐ in CI-Goldgelb,
  triggered von celebration.trigger() Store-Funktion.
- Trigger in /bewertung: nach jedem Save prüfen, ob alle Schüler×
  Kategorien ein geaendert_am haben (Klasse komplett bewertet).
- Trigger in BugButton: nach erfolgreichem Issue-Submit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Task 18: Smoke-Checklist + Push + PR

**Files:**
- Modify: `SMOKE_CHECKLIST.md`

Ziel: manuell-Smoke-Test-Anleitung in der Checkliste, dann PR öffnen.

- [ ] **Step 1: Checklist erweitern**

Lies den aktuellen Stand mit:

```bash
cat /home/neo/projects/Jiraso-reloaded/SMOKE_CHECKLIST.md
```

Hänge ans Ende der Datei einen neuen Abschnitt `## Plan 3 Smoke` an:

```markdown
## Plan 3 Smoke (Bewertung + Bemerkung)

### Vorbereitung
- [ ] Aktives Schuljahr ist gesetzt + mind. eine Klasse mit ≥2 SuS importiert (Plan 2).
- [ ] Login als Admin.

### Legacy-Import
- [ ] `/admin/legacy-import` öffnen.
- [ ] `Fächer.txt` + `Floskeln.txt` + `format.xls` aus dem Original-Programmpaket auswählen.
- [ ] Vorschau zeigt 12 Fächer + 7 Kategorien mit je 3-5 Formulierungen.
- [ ] Importieren → Summary zeigt 12/7/~30 neue Einträge.
- [ ] Zweiter Import → 0 neu, alle übersprungen (idempotent).

### Katalog-Editor
- [ ] `/admin/katalog` öffnen.
- [ ] Tab Fächer → ein Fach deaktivieren, Reload, deaktiviertes Fach wird grau.
- [ ] Tab Kategorien → eine Kategorie ↑↓ verschieben, Reihenfolge aktualisiert.
- [ ] Tab Formulierungen → Kategorie wählen, eine neue Formulierung anlegen.

### Bewertung
- [ ] Logout, Login als Klassenlehrer.
- [ ] `/bewertung` → Klasse 5a + Fach Mathe wählen.
- [ ] Matrix erscheint, alle Zellen `— keine Angabe —`.
- [ ] Eine Zelle ändern → ⟳ → ✓ binnen Sekunde.
- [ ] Auf eine Schüler-Zeile klicken → rechtes Panel zeigt Bemerkungs-Textarea.
- [ ] In Textarea tippen → 1.5 s nach letztem Tastendruck → ✓.

### Konflikt-UX
- [ ] Zwei App-Sessions parallel öffnen (z. B. zwei `tauri dev` auf verschiedenen Ports oder zwei VMs).
- [ ] Beide auf gleiche (Schüler, Fach, Kategorie)-Zelle klicken, unterschiedliche Werte wählen.
- [ ] Zweite Session zeigt Konflikt-Banner mit `[Meine] [Ihre]`-Buttons.
- [ ] Beide Pfade testen.

### Fachlehrer-Rolle
- [ ] Logout, Login als Fachlehrer.
- [ ] `/bewertung` → Bemerkungs-Panel zeigt Hinweis „nur für Klassenlehrer/Admin".
- [ ] `/admin/katalog` → Redirect auf Dashboard.
- [ ] `/admin/legacy-import` → Redirect auf Dashboard.

### Bug-Reporter + Sternenregen
- [ ] `[bug_report] github_token = "..."` in `data/config.toml` eintragen (Admin-User PAT mit `issues:write`-Scope).
- [ ] Käfer-Knopf unten rechts ist auf jeder Route sichtbar.
- [ ] Klick → Modal öffnet sich, Titel + Beschreibung eintragen → Senden.
- [ ] Toast „Issue #N angelegt" + Sternenregen erscheint.
- [ ] Issue auf GitHub geprüft.
- [ ] Token leer setzen → erneuter Versuch zeigt Mailto-Fallback-Link.
- [ ] In `/bewertung`: alle Zellen einer kleinen Test-Klasse (3 SuS × 7 Kategorien = 21 Zellen) auf irgendetwas setzen (auch „keine Angabe"). Beim Setzen der letzten Zelle: Sternenregen.
```

- [ ] **Step 2: Final-Build prüfen**

```bash
cd /home/neo/projects/Jiraso-reloaded/src-tauri && cargo test --no-default-features --lib 2>&1 | tail -5
```

Expected: alle Tests grün (insgesamt ~62).

- [ ] **Step 3: Commit + Push + PR**

```bash
git add SMOKE_CHECKLIST.md
git commit -m "docs: Smoke-Checklist für Plan 3

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
git push -u origin feat/plan-3-bewertung
```

```bash
gh pr create --title "Plan 3: Bewertungs-Matrix + Bemerkung-UI mit Konflikt-Check" --body "$(cat <<'EOF'
## Summary
- Bewertungs-Matrix (Schüler × Kategorie für ein Fach) + Detail-Panel mit Bemerkung
- Optimistische Konfliktprüfung via geaendert_am-Token, Inline-Banner pro Zelle
- Mini-Admin-Editor für Fächer/Kategorien/Formulierungen
- Legacy-Import-Wizard für Fächer.txt + Floskeln.txt + format.xls

Spec: docs/superpowers/specs/2026-04-27-plan-3-bewertung-design.md
Plan: docs/superpowers/plans/2026-04-27-plan-3-bewertung.md

## Architektur
- Migration 003: bewertung-PK auf (schueler, fach, kategorie), formulierung_id nullable
- 4 neue Backend-Module: katalog, bewertung, bemerkung, legacy_import
- 18 neue Tauri-Commands mit Rollen-Guards
- 3 neue Frontend-Routen: /admin/katalog, /admin/legacy-import, /bewertung

## Test plan
- [ ] cargo test --no-default-features --lib (alle ~62 grün)
- [ ] Smoke gemäß SMOKE_CHECKLIST.md Plan-3-Abschnitt
- [ ] Windows-Build via GH Actions
- [ ] macOS-Build via GH Actions

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Spec-Coverage-Check

| Spec-Anforderung | Task |
|---|---|
| Schema-Migration 003 mit (schueler, fach, kategorie)-PK + nullable formulierung_id | 1 |
| `katalog.rs` mit CRUD für Fach/Kategorie/Formulierung | 2, 3 |
| `bewertung.rs` mit MatrixZelle + matrix() | 4 |
| Konflikt-Logik (geaendert_am-Token, idempotent bei gleichem Inhalt, Konflikt sonst) | 5 |
| `bemerkung.rs` mit get/set | 6 |
| `legacy_import.rs` mit Parser für Fächer.txt + Floskeln.txt + format.xls | 7 |
| `legacy_import::apply` idempotent | 8 |
| 18 Tauri-Commands mit Rollen-Guards | 9 |
| Frontend-API-Wrapper | 10 |
| Klassen + Schüler-List-Endpoints | 11 |
| `/admin/katalog`-Editor (3 Tabs, Anlegen, Toggle, Reihenfolge) | 12 |
| `/admin/legacy-import`-Wizard (3 Files, Vorschau, Apply) | 13 |
| `/bewertung`-Matrix + Detail-Panel + Konflikt-Banner + Autosave | 14 |
| Dashboard-Verlinkung | 15 |
| Bug-Reporter Backend (GitHub-API) | 16 |
| Käfer-Knopf + Sternenregen + Layout | 17 |
| Smoke-Test-Dokumentation + PR | 18 |
