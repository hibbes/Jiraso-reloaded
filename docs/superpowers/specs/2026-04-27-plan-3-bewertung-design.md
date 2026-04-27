# Plan 3 — Bewertung/Bemerkung-UI mit optimistischer Konfliktprüfung

**Datum:** 2026-04-27
**Status:** Design genehmigt, Implementierung steht aus
**Vorgänger:** Plan 1 (Foundation), Plan 2 (Stammdaten-Import inkl. CSV-Erweiterung)

## Ziel

Lehrer:innen geben Verbalbeurteilungen für Schüler:innen der Klassen 5 und 6 ein. Pro (Schüler, Fach, Kategorie) wählen sie genau eine Formulierung aus dem schulinternen Katalog (oder „keine Angabe"). Pro Schüler:in erfasst die Klassenlehrer:in eine zusätzliche Freitext-Bemerkung. Mehrere Lehrer:innen arbeiten parallel; der vorhandene Session-Lock erlaubt 2 gleichzeitige Sitzungen, daher braucht es optimistische Konfliktprüfung.

## Quellen

- Legacy-Code `bewwahl.java` (Quellcodes/): pro Kategorie eine `ButtonGroup` mit JRadioButton „keine Angabe" + 3-5 mutually-exclusive JCheckBoxes → exakt 1 Wahl pro Kategorie. 7 Kategorien, persistiert als 7 Zeilen `0..N` in `<Schüler><Klasse><Fach>.txt`.
- Legacy-Code `bemeingabe.java`: 1 JTextArea, beim Speichern auf max. 7×100 Zeichen umgebrochen, gespeichert in `bem<Schüler>.txt`.
- Legacy-Daten `Floskeln.txt` (latin-1, `-` als Trenner zwischen Kategorien), `Fächer.txt` (12 Fächer, ASCII), `format.xls` (Excel-Output-Template, Kategorien-Labels in Spalte A bei Zeilen-Offsets 7-33).

## Design-Entscheidungen

| # | Entscheidung |
|---|---|
| Q1 | **Pro Kategorie genau 1 Formulierung** (oder NULL = keine Angabe). Schema-Migration 003 ändert `bewertung`-PK von `(schueler, fach, formulierung)` auf `(schueler, fach, kategorie)`. |
| Q2 | **Tabellen-Matrix**: Schüler-Zeilen × Kategorie-Spalten für ein gewähltes Fach, Bemerkung in Detail-Panel rechts für die fokussierte Zeile. |
| Q3 | **Mini-Admin + Legacy-Import**: CRUD-UI für Fächer/Kategorien/Formulierungen plus einmaliger Import-Wizard für die drei Legacy-Files. |
| Q4 | **Rollen**: Fachlehrer→Bewertung in einem gewählten Fach, Klassenlehrer→alle Fächer + Bemerkung, Administrator→alles inkl. Katalog-Editor. |
| Q5 | **Konflikt-UX**: Inline-Banner pro betroffener Zelle mit `[Meine übernehmen]` / `[Ihre behalten]`. Andere Zellen werden still refresht. |
| Q6 | **Autosave**: on-blur pro Zelle, Bemerkung 1500 ms debounced. Status-Icon pro Zelle (`✓` / `⟳` / `⚠`). Kein Speichern-Button. |

## Schema-Migration 003

```sql
-- 003_bewertung_kategorie_pk.sql
DROP TABLE IF EXISTS bewertung;

CREATE TABLE bewertung (
    schueler_id     INTEGER NOT NULL REFERENCES schueler(id) ON DELETE CASCADE,
    fach_id         INTEGER NOT NULL REFERENCES fach(id) ON DELETE CASCADE,
    kategorie_id    INTEGER NOT NULL REFERENCES kategorie(id) ON DELETE CASCADE,
    formulierung_id INTEGER REFERENCES formulierung(id) ON DELETE SET NULL,
    geaendert_am    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (schueler_id, fach_id, kategorie_id)
);

CREATE INDEX idx_bewertung_klasse_fach
    ON bewertung(fach_id, schueler_id);
```

Plan-2-Daten in `bewertung` sind leer (kein Import-Pfad bisher), DROP+CREATE ist verlustfrei.

`bemerkung` (PK `schueler_id`) bleibt unverändert; `geaendert_am` ist bereits dort.

## Backend-Module

### `katalog.rs` (neu)

CRUD pro Entity, alle skopiert auf das aktive Schuljahr:

- `fach_list(schuljahr_id) -> Vec<Fach>`
- `fach_upsert(schuljahr_id, name, reihenfolge, aktiv) -> i64`
- `fach_set_reihenfolge(id, neue) -> ()`
- `fach_set_aktiv(id, bool) -> ()`
- analog für `kategorie` und `formulierung` (Formulierung zusätzlich `kategorie_id`)

Keine harte Löschung wenn referenziert; stattdessen `aktiv = 0` (soft-disable).

### `bewertung.rs` (neu)

```rust
pub struct MatrixZelle {
    pub schueler_id: i64,
    pub kategorie_id: i64,
    pub formulierung_id: Option<i64>,
    pub geaendert_am: String,
}

pub struct BewertungUpdate {
    pub schueler_id: i64,
    pub fach_id: i64,
    pub kategorie_id: i64,
    pub formulierung_id: Option<i64>,
    pub vorheriger_stand: Option<String>, // None = neu, Some(ts) = vorhandene Zelle
}

pub enum SetResult {
    Ok { neuer_stand: String },
    Konflikt { server_formulierung_id: Option<i64>, server_geaendert_am: String },
}

pub fn matrix(conn, klasse_id, fach_id) -> Vec<MatrixZelle>;
pub fn set(conn, update) -> SetResult;
pub fn set_bemerkung(conn, schueler_id, text, vorheriger_stand) -> SetResult;
pub fn get_bemerkung(conn, schueler_id) -> (Option<String>, Option<String>); // text, geaendert_am
```

Konflikt-Logik in `set`:

```rust
let server = SELECT formulierung_id, geaendert_am
             FROM bewertung
             WHERE schueler_id=? AND fach_id=? AND kategorie_id=?;
match (update.vorheriger_stand, server) {
    (None, None) => INSERT,                              // neu
    (None, Some(s)) if s.formulierung_id == update.formulierung_id => Ok(s.geaendert_am),  // idempotent
    (None, Some(s)) => Konflikt(s),                      // wir wussten nichts, Server hat was
    (Some(ts), Some(s)) if ts == s.geaendert_am => UPDATE,  // unser Stand passt
    (Some(_), Some(s)) if s.formulierung_id == update.formulierung_id => Ok(s.geaendert_am),  // gleicher Inhalt, idempotent
    (Some(_), Some(s)) => Konflikt(s),                   // anderer Stand, anderer Inhalt
    (Some(_), None) => Konflikt(...),                    // Zelle wurde gelöscht
}
```

`formulierung_id = None` (NULL) = "keine Angabe" → DELETE der Zeile beim Update, kein INSERT bei initial-NULL.

### `legacy_import.rs` (neu)

- `parse_faecher(bytes)` → `Vec<String>` aus `Fächer.txt` (ASCII, eine pro Zeile, leere Zeilen gestrippt)
- `parse_floskeln(bytes)` → `Vec<KategorieMitFormulierungen>` aus `Floskeln.txt` (latin-1, `-` als Trenner)
- `parse_format_kategorien(bytes)` → `Vec<String>` aus `format.xls` Spalte A (Kategorie-Labels, mehrzeilig zu glätten z. B. "Beteiligung\nam\nUnterricht")
- `apply_legacy_import(conn, schuljahr_id, faecher, kategorien_namen, floskeln)`: idempotent — bestehende mit gleichem Namen werden übersprungen, neue gehängt mit nächstgrößter `reihenfolge`.

Korrelation Floskeln↔Kategorien-Labels: nach Reihenfolge (1. Floskel-Block ↔ 1. Label, etc.). Falls Anzahl ungleich → Fehler mit Hinweis.

### `commands.rs` (Erweiterung)

Neue Commands (alle behalten den `desktop`-Feature-Gate):

- `katalog_*` (~10 Commands für CRUD)
- `bewertung_matrix(klasse_id, fach_id)` — alle Rollen außer ungebunden
- `bewertung_set(update)` — Fachlehrer beschränkt auf sein Fach (Frontend setzt `fach_id`, Backend prüft per Session-State; KISS-Variante: trust frontend, da Lehrer:innen-Rolle eh trusted)
- `bemerkung_get/set` — nur Klassenlehrer + Admin
- `legacy_import_preview(faecher_bytes, floskeln_bytes, format_bytes)` → Vorschau-Struktur
- `legacy_import_apply(...)` — nur Admin

Rollen-Guards:
- `require_klassenlehrer_oder_admin()` für Bemerkungs-Commands
- `require_admin()` für Katalog-Edits + Legacy-Import (bereits vorhanden)

## Frontend-Routen

### `/admin/katalog`

Drei Tabs (Fächer | Kategorien | Formulierungen). Pro Tab eine Liste mit:
- Inline-Edit-Felder (Name)
- Drag-Handles für `reihenfolge` (HTML5 Drag&Drop reicht)
- Toggle-Switch für `aktiv`
- „+ Neu"-Button am Ende

Formulierungen-Tab gruppiert nach Kategorie (Akkordeon).

### `/admin/legacy-import`

Drei File-Picker (Fächer.txt / Floskeln.txt / format.xls), Vorschau-Tabelle mit „neu" / „bereits vorhanden", Apply-Button.

### `/bewertung` (Hauptarbeits-View)

Header:
- Klasse-Dropdown (alle Klassen des aktiven Schuljahrs)
- Fach-Dropdown bzw. Tabs (alle aktiven Fächer; Fachlehrer sieht nur das beim Login gewählte Fach)
- aktiver-User-Indikator + ggf. „Andere Sitzung: X (Klassenlehrer)"

Hauptbereich (Grid 2/3 + 1/3):

**Linke Hälfte — Matrix:**
- Sticky-Header mit Kategorie-Namen
- Eine Zeile pro Schüler:in (`Nachname, Vorname` aus `sortname`)
- Pro Zelle ein `<select>` mit `<option>— keine Angabe —</option>` + `<option v=fid>Formulierungs-Text</option>` für jede aktive Formulierung der Kategorie
- Rechts neben der ersten Spalte ein 12px-Status-Icon der gerade fokussierten Zeile (✓/⟳/⚠)
- Tab/Pfeil-Navigation; Enter öffnet Dropdown
- onChange: zeige `⟳`, sende `bewertung_set` mit `vorheriger_stand`. Bei `Ok` → `✓`. Bei `Konflikt` → `⚠`-Icon und Banner unter der Zelle: `Kollegin/Kollege hat dort gerade „<Text>" gewählt. [Meine übernehmen] [Ihre behalten]`

**Rechte Hälfte — Detail-Panel der fokussierten Schüler:in:**
- Schüler-Header + ggf. ASV-UUID (Mouseover)
- Bemerkungs-Textarea (4 Zeilen × 80 Zeichen Sichtbar, kein hartes Limit, aber Live-Counter „N/700 Zeichen")
- Bei Klassenlehrer/Admin: editierbar, debounced-Autosave 1500 ms. Bei Fachlehrer: read-only oder ausgeblendet.
- Kollisions-Banner über dem Textfeld bei Konflikt

**Background-Refresh:**
Alle 30 s pollt die Page `bewertung_matrix` und `bemerkung_get` für den fokussierten Schüler. Zellen ohne pending edit werden still upgedatet; Zellen mit pending edit bleiben unangetastet.

## Tests

Backend (alle gegen In-Memory-SQLite-Connection in `tempdir`):

```
katalog::tests::
    fach_lifecycle_create_update_disable_relist
    kategorie_reihenfolge_swap
    formulierung_pro_kategorie_isoliert
bewertung::tests::
    matrix_leer_bei_neuer_klasse
    set_neu_kein_vorheriger_stand
    set_update_mit_korrektem_token
    set_keine_angabe_loescht_zeile
    set_konflikt_anderer_inhalt_anderer_token
    set_idempotent_gleicher_inhalt
    set_konflikt_zeile_wurde_extern_geloescht
    bemerkung_konflikt
legacy_import::tests::
    parse_faecher_strippt_leerzeilen
    parse_floskeln_latin1_und_trennzeichen
    parse_format_xls_kategorienamen
    apply_idempotent_skipped_existing
    apply_meldet_anzahl_unterschied_floskeln_vs_format
```

Frontend (manueller Smoke-Test, siehe SMOKE_CHECKLIST.md-Erweiterung):

1. Admin: Katalog-Editor → 3 Fächer + 2 Kategorien + 4 Formulierungen anlegen.
2. Admin: Legacy-Import mit den 3 Files aus `verbalbeurteilung-analyse/` → 12 Fächer + 7 Kategorien + ~30 Formulierungen erkannt.
3. Klassenlehrer: `/bewertung`, Klasse 5a, Fach Mathe → Matrix erscheint, alle Zellen `— keine Angabe —`. Eine Auswahl treffen → ✓-Icon.
4. Zwei Browser-Sessions parallel auf gleiche Zelle → ⚠-Banner mit Resolution-Choice erscheint.
5. Bemerkung 750 Zeichen tippen → Counter wird rot ab 700, Save klappt trotzdem (kein hartes Limit).
6. Fachlehrer-Login: Bemerkung-Panel ausgeblendet, Fach-Tab nur aktives Fach.

## Out of Scope (Plan 3)

- PDF-Druck der Beurteilungen (Plan 5: WebView2-Print mit HTML-A4-quer-Template, Sammel-PDF Default)
- Vollständigkeitsreport „Welche SuS haben noch unbewertete Fächer?" (Plan 4)
- Audit-Trail wer-wann-was (Plan 6)
- Kategorien-pro-Fach-Override (Sport hat ggf. andere Kategorien als Mathe). Plan 3 nutzt dieselben Kategorien für alle Fächer.
- Mehrere Fachlehrer:innen mit getrennten Logins (Plan 6 evtl., aktuell einer geteilten `Fachlehrer`-Rolle).

## Implementierungs-Schritte (für `writing-plans`-Skill)

1. **Migration 003 + Schema-Tests**
2. **Katalog-Backend** (`katalog.rs` + Commands + Tests)
3. **Bewertungs-Backend** (`bewertung.rs` + Commands + Konflikt-Tests)
4. **Legacy-Import-Backend** (`legacy_import.rs` + Tests gegen real-data fixtures)
5. **Frontend Katalog-Editor** (`/admin/katalog`)
6. **Frontend Legacy-Import-Wizard** (`/admin/legacy-import`)
7. **Frontend Bewertungs-Matrix** (`/bewertung`) inkl. Konflikt-Banner und Autosave
8. **End-to-End Smoke + Doc-Updates** (SMOKE_CHECKLIST.md, README, Plan-Memory)
