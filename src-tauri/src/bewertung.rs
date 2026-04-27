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
