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
        FieldKind::Vorname => &["vorname", "rufname"],
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
}
