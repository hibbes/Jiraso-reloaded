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
