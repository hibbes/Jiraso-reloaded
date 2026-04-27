// src-tauri/src/bewertung.rs
use crate::error::AppResult;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MatrixZelle {
    pub schueler_id: i64,
    pub kategorie_id: i64,
    pub formulierung_id: Option<i64>,
    pub geaendert_am: String,
}

/// Liefert alle gespeicherten Bewertungs-Zellen für eine (Klasse, Fach)-Kombi.
/// "Keine Angabe" wird als explizite Zeile mit formulierung_id=NULL gespeichert,
/// damit "Klasse komplett bewertet" detektierbar ist (jede Zelle hat geaendert_am).
/// Fehlende Zeilen = "Lehrer:in hat sich noch nicht damit befasst".
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

/// Setzt eine Bewertungs-Zelle. Verhalten:
/// - `formulierung_id = None` → explizite NULL-Zeile (kein DELETE), damit "Klasse fertig" detektierbar ist.
/// - `vorheriger_stand = None` und Server hat keine Zeile → INSERT.
/// - `vorheriger_stand = None` und Server hat gleichen Inhalt → idempotenter OK.
/// - `vorheriger_stand = None` und Server hat anderen Inhalt → Konflikt.
/// - `vorheriger_stand = Some(ts)` und Server-ts == ts → UPDATE.
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
}
