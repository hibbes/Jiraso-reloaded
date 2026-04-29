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

/// Test-Helfer: Würfelt für eine ganze Klasse zufällige Bewertungen +
/// optional Bemerkungen. Admin-only (Tauri-Command-Wrapper prüft das).
/// `null_anteil` = Anteil der Zellen, die als „keine Angabe" eingetragen
/// werden (0.0 = keine, 1.0 = nur NULL). Üblich: 0.1.
/// Liefert (cells_geschrieben, bemerkungen_geschrieben).
pub fn wuerfle_klasse(
    conn: &mut Connection,
    klasse_id: i64,
    null_anteil: f64,
    seed: u64,
) -> AppResult<(usize, usize)> {
    let schuljahr_id: i64 = conn.query_row(
        "SELECT schuljahr_id FROM klasse WHERE id = ?1",
        params![klasse_id],
        |r| r.get(0),
    )?;

    let schueler: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM schueler WHERE klasse_id = ?1")?;
        let rows: Vec<i64> = stmt
            .query_map(params![klasse_id], |r| r.get::<_, i64>(0))?
            .collect::<Result<_, _>>()?;
        rows
    };
    let faecher: Vec<i64> = {
        let mut stmt = conn.prepare(
            "SELECT id FROM fach WHERE schuljahr_id = ?1 AND aktiv = 1 ORDER BY reihenfolge",
        )?;
        let rows: Vec<i64> = stmt
            .query_map(params![schuljahr_id], |r| r.get::<_, i64>(0))?
            .collect::<Result<_, _>>()?;
        rows
    };
    let kategorien: Vec<i64> = {
        let mut stmt = conn.prepare(
            "SELECT id FROM kategorie WHERE schuljahr_id = ?1 AND aktiv = 1 ORDER BY reihenfolge",
        )?;
        let rows: Vec<i64> = stmt
            .query_map(params![schuljahr_id], |r| r.get::<_, i64>(0))?
            .collect::<Result<_, _>>()?;
        rows
    };

    // Pro Kategorie aktive Formulierungs-IDs
    let mut form_pool: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for k in &kategorien {
        let ids: Vec<i64> = {
            let mut stmt = conn.prepare(
                "SELECT id FROM formulierung WHERE kategorie_id = ?1 AND aktiv = 1",
            )?;
            let rows: Vec<i64> = stmt
                .query_map(params![k], |r| r.get::<_, i64>(0))?
                .collect::<Result<_, _>>()?;
            rows
        };
        form_pool.insert(*k, ids);
    }

    let beispiele = beispiel_bemerkungen();

    let tx = conn.transaction()?;
    let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut cells = 0usize;
    let mut bems = 0usize;

    for s in &schueler {
        for f in &faecher {
            for k in &kategorien {
                let pool = form_pool.get(k).cloned().unwrap_or_default();
                let pick: Option<i64> = if pool.is_empty()
                    || (next() as f64 / u64::MAX as f64) < null_anteil
                {
                    None
                } else {
                    Some(pool[(next() as usize) % pool.len()])
                };
                tx.execute(
                    "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id, geaendert_am)
                     VALUES (?1, ?2, ?3, ?4, datetime('now'))
                     ON CONFLICT(schueler_id, fach_id, kategorie_id) DO UPDATE SET
                        formulierung_id = excluded.formulierung_id,
                        geaendert_am    = excluded.geaendert_am",
                    params![s, f, k, pick],
                )?;
                cells += 1;
            }
        }
        // 60% Chance auf eine Bemerkung
        if (next() as f64 / u64::MAX as f64) < 0.6 && !beispiele.is_empty() {
            let text = beispiele[(next() as usize) % beispiele.len()];
            tx.execute(
                "INSERT INTO bemerkung(schueler_id, text, geaendert_am)
                 VALUES (?1, ?2, datetime('now'))
                 ON CONFLICT(schueler_id) DO UPDATE SET
                    text = excluded.text,
                    geaendert_am = excluded.geaendert_am",
                params![s, text],
            )?;
            bems += 1;
        }
    }

    tx.commit()?;
    Ok((cells, bems))
}

fn beispiel_bemerkungen() -> Vec<&'static str> {
    vec![
        "Eine engagierte Schülerin, die sich gut in die Klassengemeinschaft einbringt und im Unterricht stets aufmerksam mitarbeitet.",
        "Ein freundlicher Schüler, der zuverlässig und gewissenhaft arbeitet. Im Bereich Konzentration sind weitere Fortschritte möglich.",
        "Hat sich im Lauf des Schuljahres deutlich entwickelt und nimmt nun aktiv am Unterrichtsgeschehen teil.",
        "Bringt sich mit guten Beiträgen und originellen Ideen ein und sorgt für ein positives Lernklima.",
        "Sollte sich darauf konzentrieren, Hausaufgaben regelmäßig und vollständig zu erledigen, um den eigenen Lernerfolg zu sichern.",
        "Verfügt über sehr gute Auffassungsgabe und arbeitet selbstständig auf hohem Niveau.",
        "Zeigt im sozialen Miteinander Verlässlichkeit und ist für Mitschüler:innen ein:e angenehme:r Lernpartner:in.",
        "Mit etwas mehr Selbstvertrauen wird auch die mündliche Beteiligung sich weiter steigern lassen.",
    ]
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
    fn wuerfle_klasse_befuellt_alle_zellen() {
        let (_d, mut conn) = seed();
        // Hier sind 1 Klasse, 2 Schüler, 1 Fach, 2 Kategorien, 2 Formulierungen
        let (cells, bems) = wuerfle_klasse(&mut conn, 1, 0.0, 42).unwrap();
        assert_eq!(cells, 4, "2 Schüler × 1 Fach × 2 Kategorien");
        let _ = bems; // 0..2 möglich (60% Bemerkungs-Chance)
        let m = matrix(&conn, 1, 1).unwrap();
        assert_eq!(m.len(), 4);
        // alle Zellen mit gültiger Formulierung (null_anteil=0)
        assert!(m.iter().all(|z| z.formulierung_id.is_some()));
    }

    #[test]
    fn wuerfle_klasse_setzt_keine_angabe_wenn_null_anteil_1() {
        let (_d, mut conn) = seed();
        let (cells, _) = wuerfle_klasse(&mut conn, 1, 1.0, 42).unwrap();
        assert_eq!(cells, 4);
        let m = matrix(&conn, 1, 1).unwrap();
        assert!(m.iter().all(|z| z.formulierung_id.is_none()));
    }

    #[test]
    fn wuerfle_klasse_ist_idempotent_pro_seed() {
        let (_d, mut conn1) = seed();
        let (_d2, mut conn2) = seed();
        wuerfle_klasse(&mut conn1, 1, 0.0, 1234).unwrap();
        wuerfle_klasse(&mut conn2, 1, 0.0, 1234).unwrap();
        let m1 = matrix(&conn1, 1, 1).unwrap();
        let m2 = matrix(&conn2, 1, 1).unwrap();
        // Beide Matrizen identisch (gleicher Seed)
        let mut m1_sorted: Vec<_> = m1.iter().map(|z| (z.schueler_id, z.kategorie_id, z.formulierung_id)).collect();
        let mut m2_sorted: Vec<_> = m2.iter().map(|z| (z.schueler_id, z.kategorie_id, z.formulierung_id)).collect();
        m1_sorted.sort();
        m2_sorted.sort();
        assert_eq!(m1_sorted, m2_sorted);
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
