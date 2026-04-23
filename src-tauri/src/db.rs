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
