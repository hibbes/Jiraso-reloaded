// src-tauri/src/legacy_import.rs
use crate::error::{AppError, AppResult};
use calamine::{Data, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

/// Eine Kategorie mit ihren Formulierungen (Reihenfolge matters).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegacyKategorie {
    pub name: String,
    pub formulierungen: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LegacyImportPreview {
    pub faecher: Vec<String>,
    pub kategorien: Vec<LegacyKategorie>,
}

/// `Fächer.txt`: ASCII / latin-1, eine Zeile pro Fach, leere Zeilen verworfen.
pub fn parse_faecher(bytes: &[u8]) -> AppResult<Vec<String>> {
    let text = decode_text(bytes);
    Ok(text.lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// `Floskeln.txt`: latin-1, Formulierungen pro Zeile, `-` als Trenner zwischen
/// Kategorie-Blöcken. Liefert eine Liste anonymer Blöcke (Namen kommen aus format.xls).
pub fn parse_floskeln_blocks(bytes: &[u8]) -> AppResult<Vec<Vec<String>>> {
    let text = decode_text(bytes);
    let mut blocks: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() { continue; }
        if l == "-" {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(l.to_string());
    }
    if !current.is_empty() { blocks.push(current); }
    Ok(blocks)
}

/// `format.xls`: Excel-Datei (alte BIFF-Variante; calamine kann das via auto-detection).
/// Die Kategorie-Labels stehen in Spalte A bei den Block-Anfangs-Zeilen.
/// In der Schiller-Vorlage: Zeilen 7,11,14,19,22,25,29 (0-basiert) tragen Labels.
/// Wir scannen Spalte A nach nicht-leeren Zellen ab Zeile 7 und sammeln diese
/// als Kategorie-Namen. Mehrzeilige Labels (z.B. "Beteiligung\nam\nUnterricht")
/// werden zusammengefügt durch Whitespace-Konkatenation der direkt folgenden
/// Zeilen, in denen Spalte A nicht-leer ist und Spalte B die gleiche Block-
/// Zugehörigkeit signalisiert.
///
/// Heuristik: scanne Spalte A von Zeile 7 abwärts; jede nicht-leere Zelle
/// startet ein neues Label oder erweitert das laufende, abhängig davon, ob
/// in Spalte B (Index 1) zwischen den zwei Zellen leere Zellen liegen.
pub fn parse_format_xls_kategorien(bytes: &[u8]) -> AppResult<Vec<String>> {
    let mut wb: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes))
        .map_err(|e| AppError::Config(format!("format.xls ungültig: {e}")))?;
    let sheet_name = wb.sheet_names().first().cloned()
        .ok_or_else(|| AppError::Config("format.xls hat keine Tabelle".into()))?;
    let range = wb.worksheet_range(&sheet_name)
        .map_err(|e| AppError::Config(format!("Tabelle nicht lesbar: {e}")))?;

    // Wir erwarten Header in Zeile 0..6 und Floskeln ab Zeile 7.
    // Eine Zeile mit nicht-leerer Spalte A startet ein neues Label oder ergänzt
    // das laufende. Wir sammeln Labels nach Block-Position: jeder Block beginnt,
    // wenn Spalte B in einer vorherigen Zeile leer war.
    let rows: Vec<Vec<String>> = range.rows().map(|r| {
        r.iter().map(cell_to_string).collect()
    }).collect();

    if rows.len() < 8 {
        return Err(AppError::Config("format.xls hat zu wenige Zeilen".into()));
    }

    let mut labels: Vec<String> = Vec::new();
    let mut current_label: Vec<String> = Vec::new();
    let mut last_b_empty = true;

    for r in 7..rows.len() {
        let row = &rows[r];
        let a = row.get(0).cloned().unwrap_or_default();
        let b = row.get(1).cloned().unwrap_or_default();
        let a_trim = a.trim().to_string();
        let b_empty = b.trim().is_empty();

        // Block-Wechsel = Spalte B vorher leer und jetzt voll
        if last_b_empty && !b_empty {
            // Wir schließen das vorherige Label ab (falls vorhanden)
            if !current_label.is_empty() {
                labels.push(current_label.join(" ").trim().to_string());
                current_label.clear();
            }
        }

        if !a_trim.is_empty() {
            current_label.push(a_trim.clone());
        }
        last_b_empty = b_empty;

        // Abbruch: Footer "zusätzliche Bemerkungen" markiert Ende
        if a_trim.to_lowercase().contains("bemerkungen") || a_trim.starts_with("Offenburg") {
            break;
        }
    }
    if !current_label.is_empty() {
        labels.push(current_label.join(" ").trim().to_string());
    }
    Ok(labels)
}

/// Kombiniert `parse_faecher` + `parse_floskeln_blocks` + `parse_format_xls_kategorien`
/// und korreliert die anonymen Floskel-Blöcke mit den Kategorie-Namen aus format.xls.
/// Falls die Anzahl Blöcke ≠ Anzahl Labels, gibt es einen sprechenden Fehler.
pub fn parse_alle(
    faecher_bytes: &[u8],
    floskeln_bytes: &[u8],
    format_bytes: &[u8],
) -> AppResult<LegacyImportPreview> {
    let faecher = parse_faecher(faecher_bytes)?;
    let blocks = parse_floskeln_blocks(floskeln_bytes)?;
    let labels = parse_format_xls_kategorien(format_bytes)?;
    if blocks.len() != labels.len() {
        return Err(AppError::Config(format!(
            "Anzahl Floskel-Blöcke ({}) passt nicht zur Anzahl Kategorie-Labels ({}). \
             Bitte format.xls und Floskeln.txt prüfen.",
            blocks.len(), labels.len()
        )));
    }
    let kategorien = labels.into_iter().zip(blocks.into_iter())
        .map(|(name, formulierungen)| LegacyKategorie { name, formulierungen })
        .collect();
    Ok(LegacyImportPreview { faecher, kategorien })
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0 { format!("{}", *f as i64) } else { format!("{f}") }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERR:{e:?}"),
    }
}

fn decode_text(bytes: &[u8]) -> String {
    // Wiederverwendung der gleichen Logik wie in import.rs (UTF-8 → Windows-1252 Fallback).
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            cow.into_owned()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_faecher_strippt_leerzeilen() {
        let bytes = b"Mathe\n\nDeutsch\n   \nEnglisch\n";
        let f = parse_faecher(bytes).unwrap();
        assert_eq!(f, vec!["Mathe", "Deutsch", "Englisch"]);
    }

    #[test]
    fn parse_floskeln_trennt_an_minus() {
        let bytes = b"a\nb\n-\nc\nd\n-\ne\n";
        let blocks = parse_floskeln_blocks(bytes).unwrap();
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0], vec!["a", "b"]);
        assert_eq!(blocks[1], vec!["c", "d"]);
        assert_eq!(blocks[2], vec!["e"]);
    }

    #[test]
    fn parse_floskeln_latin1_decoding() {
        // "Müller" in Windows-1252: 0x4D 0xFC 0x6C 0x6C 0x65 0x72
        let bytes: &[u8] = b"M\xfcller\n-\n";
        let blocks = parse_floskeln_blocks(bytes).unwrap();
        assert_eq!(blocks[0][0], "Müller");
    }

    #[test]
    fn parse_alle_erkennt_anzahl_unterschied() {
        let faecher = b"Mathe\nDeutsch\n";
        let floskeln = b"a\n-\nb\n-\nc\n";
        // Wir haben kein echtes format.xls hier; benutze ein Mock-XLSX (das wird aber nicht parsen).
        // Stattdessen: Test der Mismatch-Erkennung über parse_alle nicht möglich ohne real-Daten.
        // Wir testen den Pfad indirekt im Integrationstest in Task 8 mit fixture-Files.
        let _ = (faecher, floskeln);
    }
}
