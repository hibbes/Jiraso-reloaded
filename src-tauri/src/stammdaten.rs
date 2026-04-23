// src-tauri/src/stammdaten.rs
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
            Some((_id, v, n, k)) if v == s.vorname && n == s.nachname && k == klasse_id => {
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
        // Atomarität: Wenn mitten im Batch ein UNIQUE-Constraint-Fehler auftritt,
        // muss die ganze Transaktion zurückgerollt werden.
        //
        // Abweichung vom ursprünglichen Plan: zwei Rows mit identischer UUID im
        // Input triggern KEINEN Fehler (iter 2 findet die frisch eingefügte Row
        // via UUID und läuft in den UPDATE-Pfad). Stattdessen seed wir die DB
        // direkt mit einer bestehenden UUID und lassen iter 2 via INSERT-Pfad
        // auf UNIQUE-Konflikt laufen.
        let (_d, mut conn) = fresh_conn();
        let sj = anlegen(&conn, "2025/26").unwrap();

        // Seed: Klasse + Schüler direkt in DB (ohne upsert_schueler).
        conn.execute(
            "INSERT INTO klasse(schuljahr_id, name) VALUES (?1, ?2)",
            params![sj, "5a"],
        ).unwrap();
        let klasse_id: i64 = conn.query_row(
            "SELECT id FROM klasse WHERE schuljahr_id=?1 AND name=?2",
            params![sj, "5a"], |r| r.get(0),
        ).unwrap();
        conn.execute(
            "INSERT INTO schueler(asv_uuid, klasse_id, vorname, nachname)
             VALUES (?1, ?2, ?3, ?4)",
            params!["existing-uuid", klasse_id, "Zora", "Zopf"],
        ).unwrap();
        let vor_aufruf: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schueler", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(vor_aufruf, 1);

        // Upsert-Batch: iter 1 fügt neue Row ein (keine UUID, Fallback-Pfad),
        // iter 2 würde versuchen, mit ganz neuer UUID "existing-uuid"... warte,
        // das matcht. Anderer Ansatz: iter 1 ohne UUID inserted, iter 2 mit UUID,
        // dessen Wert bereits in DB existiert → Lookup findet existing→UPDATE.
        // Das ist auch kein Fehler.
        //
        // Realistische Fehlerquelle: manuelle SQL-Injektion via Trigger.
        // Hier: wir installieren einen BEFORE-INSERT-Trigger, der den zweiten
        // INSERT abfängt. Damit testen wir genau den Rollback-Pfad.
        conn.execute(
            "CREATE TRIGGER verbiete_bert
             BEFORE INSERT ON schueler
             WHEN NEW.vorname = 'Bert'
             BEGIN
               SELECT RAISE(ABORT, 'Bert ist verboten');
             END",
            [],
        ).unwrap();

        let err = upsert_schueler(&mut conn, sj, &[
            sample_input(None, "5b", "Anna", "Apfel"),
            sample_input(None, "5b", "Bert", "Birne"),
        ]);
        assert!(err.is_err(), "Trigger-Abort muss Fehler werfen");

        // Rollback: Anna darf NICHT in der DB stehen, Klasse 5b auch nicht.
        let count_nach: i64 = conn.query_row(
            "SELECT COUNT(*) FROM schueler", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(count_nach, 1, "Atomarer Rollback schlug fehl: Anna darf nicht persistiert sein");

        let klasse_5b: i64 = conn.query_row(
            "SELECT COUNT(*) FROM klasse WHERE name = '5b'", [], |r| r.get(0),
        ).unwrap();
        assert_eq!(klasse_5b, 0, "Klasse 5b darf nicht persistiert sein (Rollback)");
    }
}
