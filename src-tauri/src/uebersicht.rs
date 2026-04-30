// src-tauri/src/uebersicht.rs
//
// Read-only Aggregat aller Module (Bewertungen + Bemerkung) pro Schueler:in.
// Liefert die Daten in der Reihenfolge, in der sie im Druck-Layout erscheinen.

use crate::error::AppResult;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModulZelle {
    pub fach_id: i64,
    pub fach_name: String,
    pub fach_reihenfolge: i64,
    pub kategorie_id: i64,
    pub kategorie_name: String,
    pub kategorie_reihenfolge: i64,
    /// `Some(text)` wenn eine Formulierung gewaehlt ist.
    /// `None` wenn entweder explizit "Keine Angabe" gespeichert wurde
    /// oder die Zelle noch nie bewertet wurde — Unterschied im `bewertet`-Flag.
    pub formulierung_text: Option<String>,
    pub formulierung_id: Option<i64>,
    pub bewertet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchuelerUebersicht {
    pub schueler_id: i64,
    pub vorname: String,
    pub nachname: String,
    pub klasse_name: String,
    pub schuljahr_bezeichnung: String,
    pub module: Vec<ModulZelle>,
    pub bemerkung: Option<String>,
    pub bewertete_module: usize,
    pub gesamt_module: usize,
}

pub fn klassen_uebersicht(conn: &Connection, klasse_id: i64) -> AppResult<Vec<SchuelerUebersicht>> {
    let mut stmt = conn.prepare(
        "SELECT id FROM schueler WHERE klasse_id = ?1 ORDER BY sortname",
    )?;
    let ids: Vec<i64> = stmt
        .query_map(params![klasse_id], |r| r.get::<_, i64>(0))?
        .collect::<Result<_, _>>()?;
    ids.into_iter()
        .map(|id| schueler_uebersicht(conn, id))
        .collect()
}

pub fn schueler_uebersicht(conn: &Connection, schueler_id: i64) -> AppResult<SchuelerUebersicht> {
    let (vorname, nachname, klasse_name, schuljahr_bezeichnung, schuljahr_id): (String, String, String, String, i64) =
        conn.query_row(
            "SELECT s.vorname, s.nachname, kl.name, sj.bezeichnung, sj.id
             FROM schueler s
             JOIN klasse kl ON kl.id = s.klasse_id
             JOIN schuljahr sj ON sj.id = kl.schuljahr_id
             WHERE s.id = ?1",
            params![schueler_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )?;

    let mut stmt = conn.prepare(
        "SELECT
             f.id, f.name, f.reihenfolge,
             k.id, k.name, k.reihenfolge,
             fm.text, b.formulierung_id,
             b.geaendert_am
         FROM fach f
         JOIN kategorie k ON k.schuljahr_id = f.schuljahr_id AND k.aktiv = 1
         LEFT JOIN bewertung b
             ON b.fach_id = f.id AND b.kategorie_id = k.id AND b.schueler_id = ?1
         LEFT JOIN formulierung fm ON fm.id = b.formulierung_id
         WHERE f.schuljahr_id = ?2 AND f.aktiv = 1
         ORDER BY f.reihenfolge, k.reihenfolge",
    )?;

    let rows = stmt.query_map(params![schueler_id, schuljahr_id], |r| {
        let geaendert: Option<String> = r.get(8)?;
        Ok(ModulZelle {
            fach_id: r.get(0)?,
            fach_name: r.get(1)?,
            fach_reihenfolge: r.get(2)?,
            kategorie_id: r.get(3)?,
            kategorie_name: r.get(4)?,
            kategorie_reihenfolge: r.get(5)?,
            formulierung_text: r.get(6)?,
            formulierung_id: r.get(7)?,
            bewertet: geaendert.is_some(),
        })
    })?;
    let module: Vec<ModulZelle> = rows.collect::<Result<Vec<_>, _>>()?;

    let bewertete_module = module.iter().filter(|m| m.bewertet).count();
    let gesamt_module = module.len();

    let bemerkung: Option<String> = conn.query_row(
        "SELECT text FROM bemerkung WHERE schueler_id = ?1",
        params![schueler_id],
        |r| r.get(0),
    ).ok();

    Ok(SchuelerUebersicht {
        schueler_id,
        vorname,
        nachname,
        klasse_name,
        schuljahr_bezeichnung,
        module,
        bemerkung,
        bewertete_module,
        gesamt_module,
    })
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
        conn.execute("INSERT INTO fach(schuljahr_id, name, reihenfolge, aktiv) VALUES (1, 'Mathe', 1, 1)", []).unwrap();
        conn.execute("INSERT INTO fach(schuljahr_id, name, reihenfolge, aktiv) VALUES (1, 'Deutsch', 2, 1)", []).unwrap();
        conn.execute("INSERT INTO kategorie(schuljahr_id, name, reihenfolge, aktiv) VALUES (1, 'Lernbereitschaft', 1, 1)", []).unwrap();
        conn.execute("INSERT INTO kategorie(schuljahr_id, name, reihenfolge, aktiv) VALUES (1, 'Sorgfalt', 2, 1)", []).unwrap();
        conn.execute("INSERT INTO formulierung(kategorie_id, text, reihenfolge, aktiv) VALUES (1, 'fleißig', 1, 1)", []).unwrap();
        conn.execute("INSERT INTO formulierung(kategorie_id, text, reihenfolge, aktiv) VALUES (2, 'sorgfältig', 1, 1)", []).unwrap();
        (dir, conn)
    }

    #[test]
    fn liefert_alle_module_leer_wenn_nichts_bewertet() {
        let (_d, conn) = seed();
        let u = schueler_uebersicht(&conn, 1).unwrap();
        assert_eq!(u.vorname, "Anna");
        assert_eq!(u.nachname, "Apfel");
        assert_eq!(u.klasse_name, "5a");
        assert_eq!(u.schuljahr_bezeichnung, "2025/26");
        assert_eq!(u.module.len(), 4); // 2 Faecher x 2 Kategorien
        assert_eq!(u.bewertete_module, 0);
        assert_eq!(u.gesamt_module, 4);
        assert!(u.module.iter().all(|m| !m.bewertet));
        assert!(u.bemerkung.is_none());
    }

    #[test]
    fn ordnet_nach_fach_dann_kategorie_reihenfolge() {
        let (_d, conn) = seed();
        let u = schueler_uebersicht(&conn, 1).unwrap();
        assert_eq!(u.module[0].fach_name, "Mathe");
        assert_eq!(u.module[0].kategorie_name, "Lernbereitschaft");
        assert_eq!(u.module[1].fach_name, "Mathe");
        assert_eq!(u.module[1].kategorie_name, "Sorgfalt");
        assert_eq!(u.module[2].fach_name, "Deutsch");
        assert_eq!(u.module[2].kategorie_name, "Lernbereitschaft");
    }

    #[test]
    fn zeigt_gewaehlte_formulierungen() {
        let (_d, conn) = seed();
        conn.execute(
            "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (1, 1, 1, 1)",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (1, 2, 2, 2)",
            [],
        ).unwrap();
        let u = schueler_uebersicht(&conn, 1).unwrap();
        let mathe_lern = &u.module[0];
        assert_eq!(mathe_lern.fach_name, "Mathe");
        assert_eq!(mathe_lern.kategorie_name, "Lernbereitschaft");
        assert_eq!(mathe_lern.formulierung_text.as_deref(), Some("fleißig"));
        assert!(mathe_lern.bewertet);

        let deutsch_sorg = u.module.iter()
            .find(|m| m.fach_name == "Deutsch" && m.kategorie_name == "Sorgfalt")
            .unwrap();
        assert_eq!(deutsch_sorg.formulierung_text.as_deref(), Some("sorgfältig"));

        assert_eq!(u.bewertete_module, 2);
    }

    #[test]
    fn keine_angabe_zaehlt_als_bewertet() {
        let (_d, conn) = seed();
        conn.execute(
            "INSERT INTO bewertung(schueler_id, fach_id, kategorie_id, formulierung_id) VALUES (1, 1, 1, NULL)",
            [],
        ).unwrap();
        let u = schueler_uebersicht(&conn, 1).unwrap();
        let mathe_lern = &u.module[0];
        assert!(mathe_lern.bewertet);
        assert!(mathe_lern.formulierung_text.is_none());
        assert_eq!(u.bewertete_module, 1);
    }

    #[test]
    fn liefert_bemerkung() {
        let (_d, conn) = seed();
        conn.execute(
            "INSERT INTO bemerkung(schueler_id, text) VALUES (1, 'Eine schöne Bemerkung.')",
            [],
        ).unwrap();
        let u = schueler_uebersicht(&conn, 1).unwrap();
        assert_eq!(u.bemerkung.as_deref(), Some("Eine schöne Bemerkung."));
    }

    #[test]
    fn inaktive_faecher_und_kategorien_werden_uebersprungen() {
        let (_d, conn) = seed();
        conn.execute("UPDATE fach SET aktiv = 0 WHERE id = 2", []).unwrap();
        conn.execute("UPDATE kategorie SET aktiv = 0 WHERE id = 2", []).unwrap();
        let u = schueler_uebersicht(&conn, 1).unwrap();
        assert_eq!(u.module.len(), 1, "nur Mathe x Lernbereitschaft bleibt aktiv");
        assert_eq!(u.module[0].fach_name, "Mathe");
        assert_eq!(u.module[0].kategorie_name, "Lernbereitschaft");
    }

    #[test]
    fn fehlerfall_unbekannter_schueler() {
        let (_d, conn) = seed();
        let r = schueler_uebersicht(&conn, 9999);
        assert!(r.is_err());
    }

    #[test]
    fn klassen_uebersicht_liefert_alle_in_sortname_ordnung() {
        let (_d, conn) = seed();
        // Anna Apfel ist schon drin; noch zwei dazu
        conn.execute("INSERT INTO schueler(klasse_id, vorname, nachname) VALUES (1, 'Bert', 'Birne')", []).unwrap();
        conn.execute("INSERT INTO schueler(klasse_id, vorname, nachname) VALUES (1, 'Cora', 'Citro')", []).unwrap();
        let alle = klassen_uebersicht(&conn, 1).unwrap();
        assert_eq!(alle.len(), 3);
        assert_eq!(alle[0].nachname, "Apfel");
        assert_eq!(alle[1].nachname, "Birne");
        assert_eq!(alle[2].nachname, "Citro");
    }

    #[test]
    fn klassen_uebersicht_leere_klasse() {
        let (_d, conn) = seed();
        conn.execute("INSERT INTO klasse(schuljahr_id, name) VALUES (1, '5b')", []).unwrap();
        let alle = klassen_uebersicht(&conn, 2).unwrap();
        assert!(alle.is_empty());
    }
}
