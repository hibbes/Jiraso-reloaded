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

fn next_reihenfolge_fach(conn: &Connection, schuljahr_id: i64) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(reihenfolge), 0) + 1 FROM fach WHERE schuljahr_id = ?1",
        params![schuljahr_id],
        |r| r.get(0),
    )?)
}

fn next_reihenfolge_kategorie(conn: &Connection, schuljahr_id: i64) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(reihenfolge), 0) + 1 FROM kategorie WHERE schuljahr_id = ?1",
        params![schuljahr_id],
        |r| r.get(0),
    )?)
}

fn next_reihenfolge_formulierung(conn: &Connection, kategorie_id: i64) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(MAX(reihenfolge), 0) + 1 FROM formulierung WHERE kategorie_id = ?1",
        params![kategorie_id],
        |r| r.get(0),
    )?)
}

/// Default-Fächerliste aus dem ursprünglichen Jiraso 2021 (Fächer.txt).
pub const DEFAULT_FAECHER: &[&str] = &[
    "Mathematik",
    "Deutsch",
    "Religion-Ethik",
    "Erdkunde",
    "Geschichte",
    "Englisch",
    "Französisch",
    "Latein",
    "Biologie",
    "Sport",
    "Musik",
    "Bildende Kunst",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SeedSummary {
    pub neue_faecher: usize,
    pub uebersprungene_faecher: usize,
}

/// Default-Kategorien + Formulierungen aus dem ursprünglichen Jiraso 2021
/// (Floskeln.txt + format.xls). 7 Kategorien mit 27 Formulierungen gesamt.
pub const DEFAULT_KATEGORIEN: &[(&str, &[&str])] = &[
    ("Lernbereitschaft", &[
        "zeigte großes Interesse und arbeitete stets fleißig, konzentriert und ausdauernd.",
        "zeigte Interesse und arbeitete in der Regel konzentriert und ausdauernd.",
        "war nur teilweise interessiert und aufmerksam.",
        "zeigte geringes Interesse, war häufig unaufmerksam und störte oft die Konzentration anderer.",
    ]),
    ("Auffassungsgabe", &[
        "fasste anspruchsvolle Inhalte rasch und sicher auf.",
        "eignete sich neuen Stoff in angemessener Zeit an.",
        "hatte Mühe und brauchte Zeit, um neue Inhalte zu begreifen.",
    ]),
    ("Beteiligung am Unterricht", &[
        "beteiligte sich stets mit bereichernden Beiträgen.",
        "beteiligte sich regelmäßig mit meist konstruktiven Beiträgen.",
        "verfolgte den Unterricht aufmerksam, aber zurückhaltend.",
        "beteiligte sich, jedoch nicht immer sachdienlich.",
        "beteiligte sich selten am Unterricht.",
    ]),
    ("Selbstständigkeit und Kreativität", &[
        "arbeitete selbstständig und zielgerichtet.",
        "kam in der Regel gut allein zurecht.",
        "brauchte oft zusätzliche Anleitung.",
    ]),
    ("Sorgfalt", &[
        "arbeitete gewissenhaft und sorgfältig.",
        "arbeitete in der Regel ordentlich.",
        "führte Aufgaben nachlässig aus.",
    ]),
    ("Einhalten von Regeln", &[
        "hielt vereinbarte Regeln stets ein.",
        "hielt vereinbarte Regeln überwiegend ein.",
        "überschritt gelegentlich die Regeln.",
        "überschritt häufig die Regeln.",
    ]),
    ("Soziales Verhalten", &[
        "brachte sich aktiv in die Klasse ein.",
        "verhielt sich gegenüber den Mitschülern freundlich und hilfsbereit.",
        "verhielt sich den Mitschülern gegenüber angemessen.",
        "verhielt sich den Mitschülern gegenüber nicht immer angemessen.",
        "tat sich schwer, die Bedürfnisse der Mitschüler wahrzunehmen und verhielt sich teilweise rücksichtslos.",
    ]),
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FloskelnSeedSummary {
    pub neue_kategorien: usize,
    pub uebersprungene_kategorien: usize,
    pub neue_formulierungen: usize,
    pub uebersprungene_formulierungen: usize,
}

/// Legt die 7 Default-Kategorien mit ihren Formulierungen im Schuljahr an.
/// Bestehende Kategorien werden weitergenutzt; bestehende Formulierungen
/// (Text-Match) werden übersprungen, neue hinten angehängt.
pub fn seed_default_floskeln(conn: &Connection, schuljahr_id: i64) -> AppResult<FloskelnSeedSummary> {
    let mut sum = FloskelnSeedSummary {
        neue_kategorien: 0,
        uebersprungene_kategorien: 0,
        neue_formulierungen: 0,
        uebersprungene_formulierungen: 0,
    };

    for (kat_name, formulierungen) in DEFAULT_KATEGORIEN {
        let kat_id: i64 = match conn.query_row(
            "SELECT id FROM kategorie WHERE schuljahr_id = ?1 AND name = ?2",
            params![schuljahr_id, kat_name],
            |r| r.get(0),
        ) {
            Ok(id) => {
                sum.uebersprungene_kategorien += 1;
                id
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let r = next_reihenfolge_kategorie(conn, schuljahr_id)?;
                conn.execute(
                    "INSERT INTO kategorie(schuljahr_id, name, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
                    params![schuljahr_id, kat_name, r],
                )?;
                sum.neue_kategorien += 1;
                conn.last_insert_rowid()
            }
            Err(e) => return Err(e.into()),
        };

        let bestehende_texte: std::collections::HashSet<String> = {
            let mut stmt = conn.prepare("SELECT text FROM formulierung WHERE kategorie_id = ?1")?;
            let rows = stmt.query_map(params![kat_id], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<_, _>>()?
        };

        for text in *formulierungen {
            if bestehende_texte.contains(*text) {
                sum.uebersprungene_formulierungen += 1;
                continue;
            }
            let r = next_reihenfolge_formulierung(conn, kat_id)?;
            conn.execute(
                "INSERT INTO formulierung(kategorie_id, text, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
                params![kat_id, text, r],
            )?;
            sum.neue_formulierungen += 1;
        }
    }
    Ok(sum)
}

/// Legt die 12 Default-Fächer im angegebenen Schuljahr an. Bestehende
/// Fächer (Name-Match, case-sensitive) werden übersprungen, neue
/// hinten an die existierende Reihenfolge angehängt.
pub fn seed_default_faecher(conn: &Connection, schuljahr_id: i64) -> AppResult<SeedSummary> {
    let bestehend: std::collections::HashSet<String> = {
        let mut stmt = conn.prepare("SELECT name FROM fach WHERE schuljahr_id = ?1")?;
        let rows = stmt.query_map(params![schuljahr_id], |r| r.get::<_, String>(0))?;
        rows.collect::<Result<_, _>>()?
    };

    let mut sum = SeedSummary { neue_faecher: 0, uebersprungene_faecher: 0 };
    for name in DEFAULT_FAECHER {
        if bestehend.contains(*name) {
            sum.uebersprungene_faecher += 1;
            continue;
        }
        let r = next_reihenfolge_fach(conn, schuljahr_id)?;
        conn.execute(
            "INSERT INTO fach(schuljahr_id, name, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
            params![schuljahr_id, name, r],
        )?;
        sum.neue_faecher += 1;
    }
    Ok(sum)
}

pub fn upsert_fach(conn: &Connection, schuljahr_id: i64, name: &str, aktiv: bool) -> AppResult<i64> {
    if name.trim().is_empty() {
        return Err(crate::error::AppError::Config(
            "Name darf nicht leer sein".into(),
        ));
    }
    let r = next_reihenfolge_fach(conn, schuljahr_id)?;
    conn.execute(
        "INSERT INTO fach(schuljahr_id, name, reihenfolge, aktiv) VALUES (?1, ?2, ?3, ?4)",
        params![schuljahr_id, name, r, aktiv as i64],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn upsert_kategorie(conn: &Connection, schuljahr_id: i64, name: &str) -> AppResult<i64> {
    if name.trim().is_empty() {
        return Err(crate::error::AppError::Config(
            "Name darf nicht leer sein".into(),
        ));
    }
    let r = next_reihenfolge_kategorie(conn, schuljahr_id)?;
    conn.execute(
        "INSERT INTO kategorie(schuljahr_id, name, reihenfolge, aktiv) VALUES (?1, ?2, ?3, 1)",
        params![schuljahr_id, name, r],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn upsert_formulierung(conn: &Connection, kategorie_id: i64, text: &str) -> AppResult<i64> {
    if text.trim().is_empty() {
        return Err(crate::error::AppError::Config(
            "Text darf nicht leer sein".into(),
        ));
    }
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

    #[test]
    fn seed_default_faecher_legt_alle_zwoelf_an() {
        let (_d, conn) = fresh_conn();
        let sum = seed_default_faecher(&conn, 1).unwrap();
        assert_eq!(sum.neue_faecher, 12);
        assert_eq!(sum.uebersprungene_faecher, 0);
        let faecher = list_faecher(&conn, 1).unwrap();
        assert_eq!(faecher.len(), 12);
        assert_eq!(faecher[0].name, "Mathematik");
        assert_eq!(faecher[11].name, "Bildende Kunst");
        assert!(faecher.iter().all(|f| f.aktiv));
    }

    #[test]
    fn seed_default_faecher_idempotent_skippt_bestehende() {
        let (_d, conn) = fresh_conn();
        upsert_fach(&conn, 1, "Mathematik", true).unwrap();
        let sum = seed_default_faecher(&conn, 1).unwrap();
        assert_eq!(sum.neue_faecher, 11);
        assert_eq!(sum.uebersprungene_faecher, 1);
        let zweiter_lauf = seed_default_faecher(&conn, 1).unwrap();
        assert_eq!(zweiter_lauf.neue_faecher, 0);
        assert_eq!(zweiter_lauf.uebersprungene_faecher, 12);
    }

    #[test]
    fn seed_default_floskeln_legt_alle_an() {
        let (_d, conn) = fresh_conn();
        let sum = seed_default_floskeln(&conn, 1).unwrap();
        assert_eq!(sum.neue_kategorien, 7);
        assert_eq!(sum.neue_formulierungen, 27);
        let kats = list_kategorien(&conn, 1).unwrap();
        assert_eq!(kats.len(), 7);
        assert_eq!(kats[0].name, "Lernbereitschaft");
        let formulierungen = list_formulierungen(&conn, kats[0].id).unwrap();
        assert_eq!(formulierungen.len(), 4);
    }

    #[test]
    fn seed_default_floskeln_idempotent() {
        let (_d, conn) = fresh_conn();
        seed_default_floskeln(&conn, 1).unwrap();
        let zweiter = seed_default_floskeln(&conn, 1).unwrap();
        assert_eq!(zweiter.neue_kategorien, 0);
        assert_eq!(zweiter.uebersprungene_kategorien, 7);
        assert_eq!(zweiter.neue_formulierungen, 0);
        assert_eq!(zweiter.uebersprungene_formulierungen, 27);
    }

    #[test]
    fn seed_default_floskeln_haengt_an_bestehende_kategorie_an() {
        let (_d, conn) = fresh_conn();
        let kat = upsert_kategorie(&conn, 1, "Lernbereitschaft").unwrap();
        upsert_formulierung(&conn, kat, "eigene Formulierung").unwrap();
        let sum = seed_default_floskeln(&conn, 1).unwrap();
        assert_eq!(sum.uebersprungene_kategorien, 1);
        assert_eq!(sum.neue_kategorien, 6);
        let f = list_formulierungen(&conn, kat).unwrap();
        assert_eq!(f.len(), 5, "eigene + 4 Default-Formulierungen");
        assert_eq!(f[0].text, "eigene Formulierung");
    }

    #[test]
    fn upsert_lehnt_leeren_namen_ab() {
        let (_d, conn) = fresh_conn();
        assert!(upsert_fach(&conn, 1, "", true).is_err());
        assert!(upsert_fach(&conn, 1, "   ", true).is_err());
        assert!(upsert_kategorie(&conn, 1, "").is_err());
        assert!(upsert_formulierung(&conn, 1, "").is_err());
    }
}
