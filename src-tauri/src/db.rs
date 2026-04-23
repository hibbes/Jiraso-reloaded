// src-tauri/src/db.rs
use crate::error::AppResult;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::Path;

static MIGRATIONS_001: &str = include_str!("migrations/001_initial.sql");
static MIGRATIONS_002: &str = include_str!("migrations/002_schuljahr_aktiv.sql");

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(MIGRATIONS_001),
        M::up(MIGRATIONS_002),
    ])
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
}
