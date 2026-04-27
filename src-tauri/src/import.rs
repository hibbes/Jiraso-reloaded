// src-tauri/src/import.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FieldKind {
    Uuid,
    Klasse,
    Nachname,
    Vorname,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ColumnMapping {
    /// Column-Index (0-based) pro erkanntem Feld.
    pub uuid: Option<usize>,
    pub klasse: usize,
    pub nachname: usize,
    pub vorname: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DetectResult {
    /// Alle Pflicht-Felder (Klasse, Nachname, Vorname) eindeutig erkannt.
    /// UUID ist optional.
    Ok(ColumnMapping),
    /// Mehrdeutigkeit oder Fehlfeld — Frontend muss manuell zuordnen.
    Ambiguous {
        headers: Vec<String>,
        suggestions: std::collections::HashMap<FieldKind, Vec<usize>>,
    },
}

/// Normalisiert einen Header für den Vergleich: lowercase, Umlaute auf Grund-Buchstabe,
/// ohne Leerzeichen / Sonderzeichen.
///
/// Abweichung vom ursprünglichen Plan: ä→a (nicht ä→ae), ö→o, ü→u, ß→s.
/// Grund: Plan-Test `umlaute_werden_normalisiert` erwartet, dass "Klässe"
/// als "Klasse" erkannt wird. Mit ä→ae wird "klässe"→"klaesse", was das
/// Keyword "klasse" NICHT enthält. Strip auf Basis-Buchstaben ist robuster.
fn normalize(h: &str) -> String {
    h.chars()
        .flat_map(|c| match c {
            'ä' | 'Ä' => "a".chars().collect::<Vec<_>>(),
            'ö' | 'Ö' => "o".chars().collect::<Vec<_>>(),
            'ü' | 'Ü' => "u".chars().collect::<Vec<_>>(),
            'ß' => "s".chars().collect::<Vec<_>>(),
            c if c.is_alphanumeric() => c.to_lowercase().collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}

fn keywords(kind: FieldKind) -> &'static [&'static str] {
    match kind {
        FieldKind::Uuid => &["uuid", "asvuuid", "id", "schuelerid"],
        FieldKind::Klasse => &["klasse", "klassenbezeichnung", "klassestufe"],
        FieldKind::Nachname => &["nachname", "familienname", "name"],
        FieldKind::Vorname => &["vorname", "vornamen", "rufname"],
    }
}

pub fn detect_columns(headers: &[String]) -> DetectResult {
    use std::collections::HashMap;
    let normalized: Vec<String> = headers.iter().map(|h| normalize(h)).collect();
    let mut suggestions: HashMap<FieldKind, Vec<usize>> = HashMap::new();

    for kind in [FieldKind::Uuid, FieldKind::Klasse, FieldKind::Nachname, FieldKind::Vorname] {
        let mut matches: Vec<usize> = Vec::new();
        for (idx, n) in normalized.iter().enumerate() {
            if keywords(kind).iter().any(|kw| n == kw) {
                matches.push(idx);
            }
        }
        // Wenn kein exakter Treffer: erlaube „contains" als schwächere Stufe
        if matches.is_empty() {
            for (idx, n) in normalized.iter().enumerate() {
                if keywords(kind).iter().any(|kw| n.contains(kw)) {
                    matches.push(idx);
                }
            }
        }
        suggestions.insert(kind, matches);
    }

    // Pflicht: Klasse, Nachname, Vorname jeweils genau 1 Treffer
    let klasse = single_or_none(&suggestions[&FieldKind::Klasse]);
    let nachname = single_or_none(&suggestions[&FieldKind::Nachname]);
    let vorname = single_or_none(&suggestions[&FieldKind::Vorname]);
    let uuid = single_or_none(&suggestions[&FieldKind::Uuid]);

    // Wenn nachname und vorname auf dieselbe Spalte zeigen, ist das mehrdeutig
    let spalten_konflikt = matches!((nachname, vorname), (Some(a), Some(b)) if a == b);

    if let (Some(k), Some(n), Some(v)) = (klasse, nachname, vorname) {
        if !spalten_konflikt {
            return DetectResult::Ok(ColumnMapping {
                uuid, klasse: k, nachname: n, vorname: v,
            });
        }
    }

    DetectResult::Ambiguous {
        headers: headers.to_vec(),
        suggestions,
    }
}

fn single_or_none(matches: &[usize]) -> Option<usize> {
    if matches.len() == 1 { Some(matches[0]) } else { None }
}

use crate::error::{AppError, AppResult};
use crate::stammdaten::SchuelerInput;
use calamine::{Data, Reader, Xlsx};
use std::io::Cursor;

/// Erkennt das Format anhand der ersten Bytes: ZIP/XLSX-Magic `PK\x03\x04`
/// → XLSX, sonst CSV. ASV-BW liefert XLSX, Jiraso-Legacy-Export liefert
/// `;`-getrennte CSV (UTF-8 oder Windows-1252).
pub fn parse_sheet(bytes: &[u8]) -> AppResult<ParsedSheet> {
    if bytes.starts_with(b"PK\x03\x04") {
        parse_xlsx(bytes)
    } else {
        parse_csv(bytes)
    }
}

/// Parst CSV-Bytes mit auto-erkanntem Delimiter (`;` oder `,`) und
/// Encoding-Fallback UTF-8 → Windows-1252. Leere Zeilen werden verworfen.
pub fn parse_csv(bytes: &[u8]) -> AppResult<ParsedSheet> {
    let text = decode_text(bytes);
    let trimmed = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let delimiter = detect_delimiter(trimmed);

    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(trimmed.as_bytes());

    let mut iter = rdr.records();
    let header_rec = iter.next()
        .ok_or_else(|| AppError::Config("CSV ist leer".into()))?
        .map_err(|e| AppError::Config(format!("CSV-Header nicht lesbar: {e}")))?;
    let headers: Vec<String> = header_rec.iter().map(|s| s.trim().to_string()).collect();

    let mut rows: Vec<Vec<String>> = Vec::new();
    for rec in iter {
        let rec = rec.map_err(|e| AppError::Config(format!("CSV-Zeile nicht lesbar: {e}")))?;
        let row: Vec<String> = rec.iter().map(|s| s.trim().to_string()).collect();
        if row.iter().any(|c| !c.is_empty()) {
            rows.push(row);
        }
    }
    Ok(ParsedSheet { headers, rows })
}

fn decode_text(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (cow, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            cow.into_owned()
        }
    }
}

fn detect_delimiter(text: &str) -> u8 {
    let first = text.lines().next().unwrap_or("");
    let semis = first.matches(';').count();
    let commas = first.matches(',').count();
    if semis >= commas { b';' } else { b',' }
}

/// Parst XLSX-Bytes und extrahiert die erste Tabelle.
/// Erwartet eine Header-Zeile oben; weitere Zeilen sind Datensätze.
/// Leere Zeilen werden übersprungen.
pub fn parse_xlsx(bytes: &[u8]) -> AppResult<ParsedSheet> {
    let mut workbook: Xlsx<_> = calamine::open_workbook_from_rs(Cursor::new(bytes))
        .map_err(|e| AppError::Config(format!("XLSX ist ungültig: {e}")))?;
    let sheet_name = workbook.sheet_names().first().cloned()
        .ok_or_else(|| AppError::Config("XLSX enthält keine Tabelle".into()))?;
    let range = workbook.worksheet_range(&sheet_name)
        .map_err(|e| AppError::Config(format!("Tabelle '{sheet_name}' nicht lesbar: {e}")))?;
    let mut rows = range.rows();
    let header_row = rows.next()
        .ok_or_else(|| AppError::Config("XLSX ist leer".into()))?;
    let headers: Vec<String> = header_row.iter().map(cell_to_string).collect();
    let body: Vec<Vec<String>> = rows
        .map(|r| r.iter().map(cell_to_string).collect())
        .filter(|r: &Vec<String>| r.iter().any(|c| !c.trim().is_empty()))
        .collect();
    Ok(ParsedSheet { headers, rows: body })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedSheet {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
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

/// Wendet ein ColumnMapping auf rohe Zeilen an → typisierte Input-Records.
/// Zeilen, bei denen Pflichtfelder (klasse, vorname, nachname) leer sind, werden verworfen.
pub fn build_inputs(sheet: &ParsedSheet, mapping: &ColumnMapping) -> Vec<SchuelerInput> {
    sheet.rows.iter().filter_map(|row| {
        let get = |idx: usize| row.get(idx).map(|s| s.trim().to_string()).unwrap_or_default();
        let klasse = get(mapping.klasse);
        let vorname = get(mapping.vorname);
        let nachname = get(mapping.nachname);
        if klasse.is_empty() || vorname.is_empty() || nachname.is_empty() {
            return None;
        }
        let asv_uuid = mapping.uuid.and_then(|i| {
            let v = get(i);
            if v.is_empty() { None } else { Some(v) }
        });
        Some(SchuelerInput { asv_uuid, klasse, vorname, nachname })
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr(cols: &[&str]) -> Vec<String> {
        cols.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn exact_header_match() {
        let r = detect_columns(&hdr(&["ASV-UUID", "Klasse", "Nachname", "Vorname"]));
        assert_eq!(r, DetectResult::Ok(ColumnMapping {
            uuid: Some(0), klasse: 1, nachname: 2, vorname: 3,
        }));
    }

    #[test]
    fn uuid_optional() {
        let r = detect_columns(&hdr(&["Klasse", "Nachname", "Vorname"]));
        assert_eq!(r, DetectResult::Ok(ColumnMapping {
            uuid: None, klasse: 0, nachname: 1, vorname: 2,
        }));
    }

    #[test]
    fn umlaute_werden_normalisiert() {
        let r = detect_columns(&hdr(&["Klässe", "Familienname", "Rufname"]));
        assert!(matches!(r, DetectResult::Ok(_)));
    }

    #[test]
    fn ambiguous_wenn_name_doppelt() {
        let r = detect_columns(&hdr(&["Klasse", "Name", "Name"]));
        match r {
            DetectResult::Ambiguous { suggestions, .. } => {
                assert!(suggestions[&FieldKind::Nachname].len() > 1);
            }
            _ => panic!("expected ambiguous"),
        }
    }

    #[test]
    fn ambiguous_wenn_vorname_fehlt() {
        let r = detect_columns(&hdr(&["Klasse", "Nachname"]));
        assert!(matches!(r, DetectResult::Ambiguous { .. }));
    }

    use rust_xlsxwriter::Workbook;

    fn make_xlsx(rows: &[&[&str]]) -> Vec<u8> {
        let mut wb = Workbook::new();
        let ws = wb.add_worksheet();
        for (r_idx, row) in rows.iter().enumerate() {
            for (c_idx, val) in row.iter().enumerate() {
                ws.write_string(r_idx as u32, c_idx as u16, *val).unwrap();
            }
        }
        wb.save_to_buffer().unwrap()
    }

    #[test]
    fn parse_xlsx_liest_header_und_zeilen() {
        let bytes = make_xlsx(&[
            &["ASV-UUID", "Klasse", "Nachname", "Vorname"],
            &["uuid-1", "5a", "Apfel", "Anna"],
            &["uuid-2", "5a", "Birne", "Bert"],
            &["", "", "", ""],   // leere Zeile wird verworfen
            &["uuid-3", "5b", "Citro", "Cora"],
        ]);
        let sheet = parse_xlsx(&bytes).unwrap();
        assert_eq!(sheet.headers, vec!["ASV-UUID", "Klasse", "Nachname", "Vorname"]);
        assert_eq!(sheet.rows.len(), 3);
    }

    #[test]
    fn build_inputs_verwirft_zeilen_ohne_pflichtfelder() {
        let sheet = ParsedSheet {
            headers: vec!["Klasse".into(), "Nachname".into(), "Vorname".into()],
            rows: vec![
                vec!["5a".into(), "Apfel".into(), "Anna".into()],
                vec!["5a".into(), "".into(), "Bert".into()],   // Nachname leer → skip
            ],
        };
        let mapping = ColumnMapping {
            uuid: None, klasse: 0, nachname: 1, vorname: 2,
        };
        let input = build_inputs(&sheet, &mapping);
        assert_eq!(input.len(), 1);
        assert_eq!(input[0].vorname, "Anna");
    }

    #[test]
    fn parse_csv_semikolon_utf8() {
        let bytes = b"Nr.;Vornamen;Familienname;Klasse;ID\n1;Anna;Apfel;5a;uuid-1\n2;Bert;Birne;5b;uuid-2\n";
        let sheet = parse_csv(bytes).unwrap();
        assert_eq!(sheet.headers, vec!["Nr.", "Vornamen", "Familienname", "Klasse", "ID"]);
        assert_eq!(sheet.rows.len(), 2);
        assert_eq!(sheet.rows[0][1], "Anna");
        assert_eq!(sheet.rows[1][2], "Birne");
    }

    #[test]
    fn parse_csv_verwirft_leere_zeilen() {
        let bytes = b"a;b\n1;2\n;\n3;4\n";
        let sheet = parse_csv(bytes).unwrap();
        assert_eq!(sheet.rows.len(), 2);
    }

    #[test]
    fn parse_csv_windows1252_fallback() {
        // "Müller" in Windows-1252: M=0x4D ü=0xFC l=0x6C l=0x6C e=0x65 r=0x72
        let bytes = b"Klasse;Familienname;Vorname\n5a;M\xfcller;Anna\n";
        let sheet = parse_csv(bytes).unwrap();
        assert_eq!(sheet.rows[0][1], "Müller");
    }

    #[test]
    fn parse_csv_strippt_utf8_bom() {
        let mut bytes = b"\xef\xbb\xbf".to_vec();
        bytes.extend_from_slice(b"Klasse;Vorname;Nachname\n5a;Anna;Apfel\n");
        let sheet = parse_csv(&bytes).unwrap();
        assert_eq!(sheet.headers[0], "Klasse");
    }

    #[test]
    fn parse_sheet_erkennt_xlsx_via_magic() {
        let xlsx = make_xlsx(&[
            &["Klasse", "Nachname", "Vorname"],
            &["5a", "Apfel", "Anna"],
        ]);
        let sheet = parse_sheet(&xlsx).unwrap();
        assert_eq!(sheet.rows.len(), 1);
        assert_eq!(sheet.rows[0][2], "Anna");
    }

    #[test]
    fn parse_sheet_faellt_auf_csv_zurueck() {
        let csv = b"Klasse;Vorname;Nachname\n5a;Anna;Apfel\n";
        let sheet = parse_sheet(csv).unwrap();
        assert_eq!(sheet.headers.len(), 3);
        assert_eq!(sheet.rows[0][1], "Anna");
    }

    #[test]
    fn jiraso_export_format_wird_erkannt() {
        // Format aus dem Legacy-Jiraso-Export (verbal_Schueler.csv)
        let bytes = b"Nr.;Vornamen;Familienname;Klasse;ID\n\
                      1;Nailya;Abb;5c;40288081-9589db3b-0195-93c7bed8-5af6\n\
                      2;Clara Sophie;Abs;5b;40288081-9589db3b-0195-93c7b20b-52bd\n";
        let sheet = parse_csv(bytes).unwrap();
        match detect_columns(&sheet.headers) {
            DetectResult::Ok(m) => {
                assert_eq!(m.klasse, 3);
                assert_eq!(m.nachname, 2);
                assert_eq!(m.vorname, 1);
                assert_eq!(m.uuid, Some(4));
            }
            DetectResult::Ambiguous { suggestions, .. } => {
                panic!("erwartet eindeutige Erkennung, bekommen: {suggestions:?}");
            }
        }
        let mapping = match detect_columns(&sheet.headers) {
            DetectResult::Ok(m) => m,
            _ => unreachable!(),
        };
        let inputs = build_inputs(&sheet, &mapping);
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].vorname, "Nailya");
        assert_eq!(inputs[1].vorname, "Clara Sophie");
        assert_eq!(inputs[0].klasse, "5c");
        assert!(inputs[0].asv_uuid.as_deref().unwrap().starts_with("40288081-"));
    }

    #[test]
    fn build_inputs_behandelt_leere_uuid_als_none() {
        let sheet = ParsedSheet {
            headers: vec!["UUID".into(), "Klasse".into(), "Nachname".into(), "Vorname".into()],
            rows: vec![
                vec!["uuid-1".into(), "5a".into(), "Apfel".into(), "Anna".into()],
                vec!["".into(), "5a".into(), "Birne".into(), "Bert".into()],
            ],
        };
        let mapping = ColumnMapping {
            uuid: Some(0), klasse: 1, nachname: 2, vorname: 3,
        };
        let input = build_inputs(&sheet, &mapping);
        assert_eq!(input[0].asv_uuid, Some("uuid-1".into()));
        assert_eq!(input[1].asv_uuid, None);
    }
}
