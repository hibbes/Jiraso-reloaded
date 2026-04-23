// src-tauri/src/stammdaten.rs
#![allow(unused_imports)]
use crate::error::AppResult;
use rusqlite::{params, Connection, OptionalExtension};
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
