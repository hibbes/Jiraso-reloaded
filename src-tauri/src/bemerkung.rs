// src-tauri/src/bemerkung.rs
use crate::error::AppResult;
use crate::bewertung::SetResult;
use rusqlite::{params, Connection};

/// Liefert (text, geaendert_am, editor_kuerzel). `None` wenn noch keine Bemerkung gespeichert.
pub fn get(conn: &Connection, schueler_id: i64) -> AppResult<Option<(String, String, Option<String>)>> {
    let row: Option<(String, String, Option<String>)> = conn.query_row(
        "SELECT text, geaendert_am, editor_kuerzel FROM bemerkung WHERE schueler_id = ?1",
        params![schueler_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).ok();
    Ok(row)
}

pub fn set(
    conn: &mut Connection,
    schueler_id: i64,
    text: &str,
    vorheriger_stand: Option<String>,
    editor_kuerzel: Option<String>,
) -> AppResult<SetResult> {
    let tx = conn.transaction()?;
    let server: Option<(String, String, Option<String>)> = tx.query_row(
        "SELECT text, geaendert_am, editor_kuerzel FROM bemerkung WHERE schueler_id = ?1",
        params![schueler_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).ok();

    let inhalt_gleich = matches!(&server, Some((t, _, _)) if t == text);

    let token_passt = match (&vorheriger_stand, &server) {
        (None, None) => true,
        (Some(ts), Some((_, srv_ts, _))) if ts == srv_ts => true,
        _ => false,
    };

    if !token_passt && !inhalt_gleich {
        let (_, sts, skz) = server.clone().unwrap_or_default();
        tx.rollback()?;
        return Ok(SetResult::Konflikt {
            server_formulierung_id: None, // Bemerkung hat keine Formulierung; Frontend liest server_geaendert_am + holt Text via get()
            server_geaendert_am: sts,
            server_editor_kuerzel: skz,
        });
    }

    if inhalt_gleich {
        let (_, ts, _) = server.unwrap();
        tx.rollback()?;
        return Ok(SetResult::Ok { neuer_stand: ts });
    }

    if text.trim().is_empty() {
        tx.execute("DELETE FROM bemerkung WHERE schueler_id = ?1", params![schueler_id])?;
        tx.commit()?;
        return Ok(SetResult::Ok { neuer_stand: String::new() });
    }

    tx.execute(
        "INSERT INTO bemerkung(schueler_id, text, editor_kuerzel, geaendert_am)
         VALUES (?1, ?2, ?3, datetime('now'))
         ON CONFLICT(schueler_id) DO UPDATE SET
            text = excluded.text,
            editor_kuerzel = excluded.editor_kuerzel,
            geaendert_am = excluded.geaendert_am",
        params![schueler_id, text, editor_kuerzel],
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
        let r = set(&mut conn, 1, "Eine schöne Bemerkung.", None, Some("MZ".into())).unwrap();
        assert!(matches!(r, SetResult::Ok { .. }));
        let (text, _ts, kz) = get(&conn, 1).unwrap().unwrap();
        assert_eq!(text, "Eine schöne Bemerkung.");
        assert_eq!(kz, Some("MZ".into()));
    }

    #[test]
    fn set_konflikt_bei_alter_token() {
        let (_d, mut conn) = seed();
        set(&mut conn, 1, "v1", None, Some("MZ".into())).unwrap();
        let r = set(&mut conn, 1, "v2", Some("alter-token-1900-01-01 00:00:00".into()), Some("NN".into())).unwrap();
        match r {
            SetResult::Konflikt { server_editor_kuerzel, .. } => {
                assert_eq!(server_editor_kuerzel, Some("MZ".into()));
            }
            _ => panic!("erwartete Konflikt"),
        }
    }

    #[test]
    fn set_idempotent_gleicher_inhalt() {
        let (_d, mut conn) = seed();
        let r1 = set(&mut conn, 1, "v1", None, None).unwrap();
        let token = match r1 { SetResult::Ok { neuer_stand } => neuer_stand, _ => unreachable!() };
        // Anderer Token, gleicher Inhalt -> idempotent
        let r2 = set(&mut conn, 1, "v1", Some("falsch".into()), None).unwrap();
        match r2 {
            SetResult::Ok { neuer_stand } => assert_eq!(neuer_stand, token),
            _ => panic!("erwartete Ok"),
        }
    }

    #[test]
    fn set_leerer_text_loescht_eintrag() {
        let (_d, mut conn) = seed();
        let r1 = set(&mut conn, 1, "v1", None, None).unwrap();
        let token = match r1 { SetResult::Ok { neuer_stand } => neuer_stand, _ => unreachable!() };
        set(&mut conn, 1, "", Some(token), None).unwrap();
        assert!(get(&conn, 1).unwrap().is_none());
    }
}
