# Jiraso-reloaded — Design-Dokument

**Status**: Design-Entwurf, abgestimmt mit dem Auftraggeber (Lehrkraft am Schiller-Gymnasium Offenburg).
**Datum**: 2026-04-21
**Autor**: gemeinsam mit Claude (Opus 4.7) erarbeitet via `superpowers:brainstorming`

## Kontext

Das bestehende Tool **Jiraso** (Java-digital-rating-solution) wurde von einem ehemaligen Schüler (Peer Schlieker) geschrieben und wird seit 2021 am Schiller-Gymnasium Offenburg für die Erstellung der **Verbalbeurteilungen für die Klassen 5 und 6** verwendet. Diese Beurteilungen werden als Zeugnis-Beilagenblatt ausgedruckt und dem Halbjahreszeugnis beigelegt.

Die Legacy-App ist in Java/Swing geschrieben (~7.660 LOC, GUI via NetBeans-Matisse-GroupLayout, JAR mit launch4j zur `Jiraso.exe` gewrappt), läuft vom Verwaltungs-Netzlaufwerk ohne Installation.

**Kernprobleme der Legacy-Version**:

- Kein Vollbild möglich (`setResizable(false)` + pixelgenaues Layout).
- Drei Klartext-Passwörter im Java-Code (`frisci`/`leodavin`/`ludvanbee`).
- Kein Audit-Trail, wer wann was geändert hat.
- Hart codierte Limits (14 Klassen, 13 Fächer, ~32 Schüler:innen/Klasse, 8 Fach-Spalten).
- Plain `FileReader`/`FileWriter` ohne Encoding-Angabe → Umlaute gehen bei Cross-Platform kaputt.
- Excel-Template via `jxl`-Library (seit ~10 Jahren nicht mehr gepflegt) gemischt mit Apache POI.
- Mehrere Popup-Fenster (`JFrame`) statt In-App-Navigation.
- Kein Lock-Mechanismus → parallele Zugriffe können sich gegenseitig überschreiben.
- Massives Copy-Paste (z. B. 14× identische `jRadioButtonNAncestorAdded`-Handler).

## Entscheidungen aus dem Brainstorming

| Thema | Entscheidung |
|---|---|
| **Deployment** | Lokal vom Tauschlaufwerk gestartet, max. 2 parallele Nutzer, Datei-Lock akzeptabel |
| **Zielbetriebssystem** | Windows-only (wie Legacy) |
| **Scope** | Feature-Parität + Quality-of-Life-Features (PDF statt xls, Klassen-Fortschritt, Suche, Backup) |
| **Runtime** | Tauri (Rust-Backend + Svelte/TypeScript-Frontend im WebView2) |
| **Authentifizierung** | 3 Rollen, je ein gehashtes Passwort (Argon2id), kein Audit pro Person (KISS) |
| **Ausgabeformat** | PDF als Sammel-Dokument, batchdruckbar, ein Blatt pro Schüler:in (A4 quer). Kein xls-Export in V1. |
| **Stammdaten-Import** | XLSX-Datei (ASV-BW-Export-Template `.exf` wird mitgeliefert), mit ASV-UUID als Stable-Key |
| **Backup-Strategie** | Tägliches automatisches Backup der SQLite-DB, 30 Tage Retention |
| **Extras** | Pädagogisches Tageszitat / Fun-Fact / Fortschritts-Feier beim Login (abschaltbar) |

## Architektur

### Grobüberblick

```
┌──────────────────────────────────────────────────────────────────┐
│ Netzlaufwerk (Tauschlaufwerk Verwaltung)                         │
│                                                                  │
│   Jiraso-reloaded/                                               │
│   ├── Jiraso-reloaded.exe     ← ein einziges Binary (~15 MB)     │
│   ├── config.toml             ← Passwort-Hashes, Pfade, Schuljahr│
│   ├── data/                                                      │
│   │   ├── jiraso.db           ← SQLite (alle Daten)              │
│   │   ├── jiraso.db.lock      ← Lock-Datei (max. 2 Nutzer)       │
│   │   └── backups/            ← jiraso-YYYY-MM-DD.db, 30 Tage    │
│   ├── templates/                                                 │
│   │   └── beilagenblatt.html  ← HTML/CSS-Druckvorlage            │
│   ├── import/                                                    │
│   │   ├── alleSchueler.xlsx   ← Admin legt hier Stammdaten rein  │
│   │   └── asv-export-template.exf  ← ASV-BW-Exportvorlage        │
│   ├── export/                                                    │
│   │   └── 2026_Sommer/        ← fertige PDFs, datiert pro Lauf   │
│   └── config/                                                    │
│       └── zitate.toml         ← Tageszitate/Fun-Facts (Goodies)  │
└──────────────────────────────────────────────────────────────────┘
     ▲                                      ▲
     │ Doppelklick öffnet                  │ Doppelklick öffnet
     │                                      │
┌─────────┐                          ┌─────────┐
│ Rechner │                          │ Rechner │
│   A     │                          │   B     │
│(Lehrer) │                          │(Admin)  │
└─────────┘                          └─────────┘
```

### Binary-Inneres (Tauri)

- **Rust-Backend** (~20 % des Codes): File-I/O, SQLite, Lock-Mechanismus, PDF-Rendering, Excel-Import (`calamine`-crate), Argon2-Hashing, Template-Rendering (`minijinja`).
- **Frontend** (Svelte + TypeScript, ~80 %): gesamte UI im WebView2.
- **Build-Output**: ein signiertes `Jiraso-reloaded.exe`.
- **Ordner-Konvention**: `config.toml` (Root-Ebene) hält die *App*-Konfiguration (Passwort-Hashes, aktuelles Schuljahr, Pfade, Goodies-Opt-out). Der Unterordner `config/` enthält *inhaltliche Assets* (`zitate.toml` mit den Goodies-Texten). Das ist bewusst getrennt, damit Admins inhaltliche Assets austauschen können, ohne an der App-Konfiguration zu rühren.

### Warum SQLite statt flacher Dateien

- Datei-Lock auf *einer* Datei ist auf SMB-Netzlaufwerken stabil; Multi-File-Konsistenz bricht bekanntlich.
- Abfragen wie „welche Kolleg:innen haben in 5b noch nicht alle bewertet?" werden trivial.
- Eine Datei lässt sich atomar sichern/zurückrollen.
- SQLite mit Single-Writer-Lock ist bei max. 2 Clients völlig unkritisch.

### Warum Templates/Import als Dateien außerhalb der DB

- Admins sollen Formulierungen oder das Druckbild anpassen können, ohne die Binary neu zu bauen und ohne in SQLite zu pfuschen.
- `alleSchueler.xlsx` kommt aus ASV-BW, der Import-Ordner wird einmal pro Schuljahr geleert und neu befüllt.

## Datenmodell

```sql
-- Stammdaten, werden einmal pro Schuljahr aus alleSchueler.xlsx importiert
CREATE TABLE schuljahr (
    id          INTEGER PRIMARY KEY,
    bezeichnung TEXT NOT NULL UNIQUE,    -- "2025/26"
    aktiv       INTEGER NOT NULL DEFAULT 0  -- genau eins aktiv
);

CREATE TABLE klasse (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,           -- "5a", "6c"
    UNIQUE(schuljahr_id, name)
);

CREATE TABLE schueler (
    id          INTEGER PRIMARY KEY,
    asv_uuid    TEXT UNIQUE,                 -- 36-stellige UUID aus ASV-BW, nullable
    klasse_id   INTEGER NOT NULL REFERENCES klasse(id) ON DELETE CASCADE,
    vorname     TEXT NOT NULL,
    nachname    TEXT NOT NULL,
    sortname    TEXT NOT NULL GENERATED ALWAYS AS (nachname || ', ' || vorname) STORED
);
CREATE INDEX idx_schueler_klasse ON schueler(klasse_id, nachname);

-- Konfigurations-Kataloge (aus Floskeln.txt / Fächer.txt geladen)
CREATE TABLE fach (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,          -- "Mathematik"
    reihenfolge  INTEGER NOT NULL,       -- für Spaltenposition im Ausdruck
    aktiv        INTEGER NOT NULL DEFAULT 1,  -- Soft-Delete: 0 = in UI nicht mehr auswählbar
    UNIQUE(schuljahr_id, name)
);

CREATE TABLE kategorie (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,          -- "Lernbereitschaft", "Auffassungsgabe", ...
    reihenfolge  INTEGER NOT NULL,
    aktiv        INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE formulierung (
    id           INTEGER PRIMARY KEY,
    kategorie_id INTEGER NOT NULL REFERENCES kategorie(id) ON DELETE CASCADE,
    text         TEXT NOT NULL,          -- "zeigte großes Interesse und arbeitete..."
    reihenfolge  INTEGER NOT NULL,       -- innerhalb der Kategorie
    aktiv        INTEGER NOT NULL DEFAULT 1
);

-- Bewertungen (Fachlehrer:in -> Schüler:in -> Formulierung)
-- Eine Zeile pro gewählter Formulierung; fehlende Zeile = nicht angekreuzt
CREATE TABLE bewertung (
    schueler_id     INTEGER NOT NULL REFERENCES schueler(id) ON DELETE CASCADE,
    fach_id         INTEGER NOT NULL REFERENCES fach(id) ON DELETE CASCADE,
    formulierung_id INTEGER NOT NULL REFERENCES formulierung(id) ON DELETE CASCADE,
    geaendert_am    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (schueler_id, fach_id, formulierung_id)
);

-- Bemerkung (Klassenlehrer:in -> Schüler:in, 1:1)
CREATE TABLE bemerkung (
    schueler_id   INTEGER PRIMARY KEY REFERENCES schueler(id) ON DELETE CASCADE,
    text          TEXT NOT NULL,         -- max 6 Zeilen x 100 Zeichen (UI-Constraint)
    geaendert_am  TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Schema-Version für spätere Migrationen
CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
INSERT INTO schema_version VALUES (1);
```

### Begründungen

- **Pro Schuljahr eigene `fach`/`kategorie`/`formulierung`-Zeilen**: Wenn Kolleg:innen nächstes Jahr eine Formulierung umschreiben, bleiben alte Bewertungen referenziell auf ihren Original-Text bezogen. Kein Daten-Bitrot bei späteren Ausdrucken.
- **`aktiv`-Flag auf `schuljahr`**: Das laufende Schuljahr ist genau eines, alte bleiben read-only. Admin klickt im nächsten Juli „Schuljahr abschließen" → neues aktiv.
- **`bewertung` als Zeile-pro-Haken** (nicht wie Legacy Ziffer 0–4): Erlaubt Mehrfach-Kreuze pro Kategorie und kollidiert nicht mit „Kategorie hat 3 oder 5 Formulierungen".
- **`geaendert_am` pro Schreib-Tupel**: Keine User-Attribution (KISS), aber wenigstens Zeitstempel für „oh, meine Änderung ist weg"-Situationen.
- **Kein `lehrer`-Tabelle**: Bei 3 geteilten Passwörtern weiß die App nicht, wer sitzt. Passt zu KISS.
- **Bemerkung-Limit 6×100 Zeichen nur UI-seitig**, nicht im Schema — leichter lockerbar später.
- **`asv_uuid` als UNIQUE, nullable**: Re-Import identifiziert vorhandene Schüler:innen an der UUID, Namensänderungen werden propagiert. Manuell angelegte Schüler:innen (Mittjahreszugänge) haben noch keine UUID und werden per `(vorname, nachname, klasse)`-Fallback gematcht.
- **`aktiv`-Spalte auf `fach`, `kategorie`, `formulierung`**: Soft-Delete. Ein vom Admin entfernter Eintrag wird nicht mehr in der Bewertungs-UI angeboten, bleibt aber für bestehende Bewertungen/alte Ausdrucke referenzierbar. Hard-Delete nur durch explizite Zweitbestätigung im Admin-Screen.

## Benutzeroberfläche

### Fenster-Verhalten

- Single-Window-App, startet maximiert (nicht echter Fullscreen mit verstecktem Chrome — das nervt beim Drucker-Dialog).
- F11 schaltet auf Borderless-Fullscreen (Beamer-Demo), ESC raus.
- Keine Popup-Fenster wie in Legacy; In-App-Router navigiert zwischen Screens.
- Fester Header: Schullogo links, Titel + Schuljahr mittig, Rolle + Logout rechts.

### Navigations-Flow

```
                        ┌──────────────────┐
                        │     LOGIN        │
                        │  Passwort [____] │
                        │  [Einloggen]     │
                        └────────┬─────────┘
                                 │ hash(pw) == stored_hash?
                        ┌────────▼─────────┐
                        │  DASHBOARD       │
                        │  (rollenabhängig)│
                        │  + Tageszitat    │
                        └──┬───┬───┬───┬───┘
     ┌─────────────────────┘   │   │   └─────────────────────┐
     │           ┌─────────────┘   └──────────┐              │
     │           │                            │              │
     ▼           ▼                            ▼              ▼
 BEWERTUNG   BEMERKUNG                    ÜBERSICHT      DRUCKEN
 eingeben    eingeben                     (readonly)     & DATEN
(Fach+Kl.)   (Kl. nur)                    (Kl. nur)      (Kl./Admin)
     │           │                            │              │
     ▼           ▼                            ▼              ▼
 Schüler-    Schüler-                     Schüler-      Filter:
 Picker      Picker                       Picker        Alle/Stufe/
     │           │                            │         Klasse/Einzel
     ▼           ▼                            ▼              │
 Kreuz-      Textfeld                     Kreuz-        ┌────▼────┐
 Matrix      (6×100                       Matrix        │VORSCHAU │
 (alle Kat.  Zeichen-                     read-only +   │(PDF)    │
  × gewähl-  Counter)                     Bemerkungs-   └────┬────┘
  tes Fach)                               feld editbar       │
     │           │                            │              ▼
     └───────────┴──── [◀ vor | nächster ▶] ──┘         [Drucken] oder
                       Auto-Save beim Wechsel             [PDF speichern]
```

### UX-Verbesserungen gegenüber Legacy

1. **Kreuz-Matrix** statt einzelner RadioButton-Panels — alle Kategorien auf einem Screen.
2. **Tastatur-Kürzel** für Schüler:in-Wechsel: Pfeiltasten `←`/`→` und zusätzlich `J`/`K` (Vim-Stil) — beides ist gleichzeitig aktiv.
3. **Fortschritts-Badges**: grünes Häkchen im Schüler-Picker, wenn für die aktuelle Klasse+Fach-Kombi alles bewertet ist. Klassenlehrer-Dashboard zeigt Aggregate („5b: 14/28 Bewertungen komplett").
4. **Auto-Save** beim Wechsel ohne Dialog; kleiner Status unten rechts („Zuletzt gespeichert 14:03:22").
5. **Datum als deutsches Format** `TT.MM.JJJJ` (Date-Picker), beim Druck-Screen vorausgefüllt.
6. **Live-PDF-Preview** im Druck-Screen: das erste Blatt wird sofort gerendert, bevor gedruckt wird.
7. **Übersicht-Screen**: alle Häkchen aller Fachlehrer:innen + editbare Bemerkung auf einer Seite.

## Admin-Screen: Formulierungen & Fächer verwalten

Ein Menüpunkt im Admin-Dashboard (für Klassenlehrer:innen nicht sichtbar, weil schul-organisatorische Entscheidungen). Zweck: Formulierungen, Kategorien und Fächer komfortabel editieren, ohne dass jemand in der DB pfuschen oder Textdateien neu importieren muss.

### Layout

```
┌─ Formulierungen & Fächer · Schuljahr 2025/26 ────────────────────┐
│                                                                   │
│  [ Fächer ]  [ Kategorien & Formulierungen ]  [ Vorschau ]       │
│                                                                   │
│  Lernbereitschaft                               [↑][↓] [umbenen.]│
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │ ≡ zeigte großes Interesse und arbeitete stets fleißig, ...  │ │
│  │   [Bearbeiten] [Löschen (wird benutzt: 234×)]                │ │
│  │ ≡ zeigte Interesse und arbeitete in der Regel konzentri...  │ │
│  │   [Bearbeiten] [Löschen]                                     │ │
│  │ ≡ war nur teilweise interessiert und aufmerksam.            │ │
│  │   [Bearbeiten] [Löschen]                                     │ │
│  │                                                              │ │
│  │ [+ Formulierung in „Lernbereitschaft" hinzufügen]            │ │
│  └─────────────────────────────────────────────────────────────┘ │
│                                                                   │
│  Auffassungsgabe                                [↑][↓] [umbenen.]│
│  ...                                                              │
│                                                                   │
│  [+ Neue Kategorie]                                               │
│                                                                   │
│  [Änderungen speichern]  [PDF-Vorschau (Beispiel-Schüler:in)]    │
└───────────────────────────────────────────────────────────────────┘
```

### Features

1. **Inline-Edit** per Doppelklick auf den Text.
2. **Drag-Drop-Sortierung** (`≡`-Griff links) für Reihenfolge der Formulierungen innerhalb einer Kategorie und der Kategorien untereinander.
3. **Neue Formulierung hinzufügen**: Knopf pro Kategorie, fügt leere Zeile im Edit-Modus ein.
4. **Neue Kategorie anlegen / umbenennen / löschen** (Löschen nur, wenn keine Formulierungen drin sind oder nach Safe-Delete-Dialog).
5. **Safe-Delete** beim Löschen einer *benutzten* Formulierung / Kategorie / Fach:
   > „Diese Formulierung wurde bisher 234× als Kreuz gesetzt. Was soll passieren?
   > - [Alle Kreuze ebenfalls löschen] — die Schüler:innen haben diese Formulierung dann nicht mehr
   > - [Formulierung nur deaktivieren] (Soft-Delete, Default) — bestehende Kreuze bleiben, für neue Bewertungen nicht mehr wählbar
   > - [Abbrechen]"
6. **Live-Vorschau** als PDF mit einem Demo-Schüler („Max Mustermann / 5x") und zufälligen Häkchen — zeigt sofort, ob eine zu lange Formulierung das Matrix-Layout bricht.
7. **Fächer analog**: gleicher Screen, Tab-Wechsel oben. Fach hinzufügen / umbenennen / (soft-)löschen, Drag-Drop für die Spalten-Reihenfolge im Ausdruck.

### Schuljahr-Kopie

Beim Jahreswechsel („Neues Schuljahr anlegen") werden Fächer, Kategorien und Formulierungen aus dem Vorjahr kopiert (nur aktive). Danach editierbar. Spart das jährliche Neu-Eintippen.

### Bewusste Einschränkungen

- **Editieren der `reihenfolge` wirkt nur auf neue Ausdrucke.** Bestehende PDFs im `export/`-Ordner werden nicht regeneriert.
- **Keine Versionierung** einzelner Formulierungs-Texte. Wer eine Formulierung umformuliert, riskiert, dass alte Bewertungen im Re-Print mit dem neuen Text erscheinen. Das ist akzeptabel, weil Ausdrucke sowieso als fertige PDFs im Export-Ordner persistiert sind.

## Locking & Mehrbenutzer

Zwei Ebenen, beide Dateien neben der DB:

### Session-Lock (`jiraso.db.lock`)

JSON-Datei, beim Programmstart geschrieben, beim regulären Beenden gelöscht:

```json
{
  "sessions": [
    {"id": "a7f3", "rechner": "VW-PC-12", "rolle": "Klassenlehrer", "since": "2026-04-21T14:03:11"},
    {"id": "b2e9", "rechner": "VW-PC-07", "rolle": "Fachlehrer",    "since": "2026-04-21T14:05:47"}
  ]
}
```

- Beim Start: Datei lesen, Einträge älter als 8 Stunden verwerfen (Crash-Recovery), eigenen Eintrag anhängen.
- 2 aktive Sessions → neuer Start blockiert mit klarer Meldung („Aktuell sind … angemeldet").
- Admin-Passwort kann Lock aufbrechen.
- SQLite-eigener Datei-Lock schützt DB-Integrität unabhängig von der Session-Lock-Logik.

### Datensatz-Konflikterkennung

Optimistisch — **Last-Writer-Wins mit Warnung**. Beim Save wird `geaendert_am` gegen den beim Laden gelesenen Wert verglichen. Konflikt → Dialog:

> „Während du bearbeitet hast, hat jemand anderes Berres, Greta / Mathematik gespeichert. Deine Änderungen überschreiben die fremden. Trotzdem speichern? [Ja, überschreiben] [Nein, erst ansehen] [Abbrechen]"

„Nein, erst ansehen" → Side-by-Side-Vergleich.

**Warum nicht pessimistisch pro Datensatz**: Zwei Fachlehrer:innen kollidieren praktisch nie beim selben Schüler/Fach (A macht Mathe, B Englisch). Optimistisch ist billiger und bricht nichts.

### Was explizit nicht im Scope ist

- Kein Realtime-Sync, kein „anderer Nutzer ist gerade drin"-Indikator.
- Keine Offline-/Merge-Logik.
- Kein Rollback-/Undo-History (Backup-File ist der Undo).

### Backup

- Beim ersten Start jedes Tages: `jiraso.db` → `data/backups/jiraso-YYYY-MM-DD.db`.
- 30 Backups behalten, ältere rotieren.
- Admin-UI: „Backup jetzt erzeugen" + „Aus Backup zurückrollen" (mit deutlicher Warnung).

## Ausgabe-Generator (PDF)

### Pipeline

```
SQLite → HTML-Template (Jinja2) → WebView2-Print → PDF → (optional) Drucker
```

Das Template `templates/beilagenblatt.html` beschreibt **eine** A4-Querseite. Für Batch werden N Kopien mit `page-break-after: always` konkateniert und in einem Rutsch zu PDF gerendert.

### Template-Auszug

```html
<!DOCTYPE html>
<html lang="de">
<head>
  <meta charset="UTF-8">
  <style>
    @page { size: A4 landscape; margin: 15mm 12mm; }
    body  { font-family: "Segoe UI", sans-serif; font-size: 9pt; color: #000; }
    .sheet { page-break-after: always; }
    .header { display: flex; justify-content: space-between; border-bottom: 1pt solid #000; padding-bottom: 4mm; }
    .student { font-size: 14pt; font-weight: bold; }
    table.matrix { width: 100%; border-collapse: collapse; margin-top: 2mm; }
    table.matrix th, table.matrix td { border: 0.5pt solid #555; padding: 1.2mm 1.5mm; }
    table.matrix th.subject { writing-mode: vertical-rl; font-size: 8pt; height: 28mm; white-space: nowrap; }
    table.matrix td.mark { text-align: center; width: 10mm; }
    .note-box { border: 0.5pt solid #555; min-height: 28mm; padding: 2mm; white-space: pre-wrap; }
    .sig-line { border-top: 0.3pt solid #000; min-width: 70mm; padding-top: 1mm; text-align: center; }
  </style>
</head>
<body>
  {% for sch in schueler_liste %}
  <div class="sheet">
    <div class="header">
      <div>Schiller-Gymnasium<br>Offenburg</div>
      <div><div class="student">{{ sch.nachname }}, {{ sch.vorname }}</div>
           <div>Allgemeine Beurteilung</div></div>
    </div>
    <div><b>Schuljahr:</b> {{ schuljahr }} | <b>Klasse:</b> {{ sch.klasse }}</div>
    <table class="matrix">
      <thead>
        <tr>
          <th></th><th></th>
          {% for f in faecher %}<th class="subject">{{ f.name }}</th>{% endfor %}
        </tr>
      </thead>
      <tbody>
        {% for kat in kategorien %}{% for formulierung in kat.formulierungen %}
        <tr>
          {% if loop.first %}<th rowspan="{{ kat.formulierungen|length }}">{{ kat.name }}</th>{% endif %}
          <td>{{ formulierung.text }}</td>
          {% for f in faecher %}
            <td class="mark">{% if sch.hakchen[(formulierung.id, f.id)] %}X{% endif %}</td>
          {% endfor %}
        </tr>
        {% endfor %}{% endfor %}
      </tbody>
    </table>
    <div><b>Zusätzliche Bemerkungen:</b></div>
    <div class="note-box">{{ sch.bemerkung }}</div>
    <div style="display:flex;justify-content:space-between;margin-top:8mm">
      <div class="sig-line">Offenburg, den {{ druck_datum }}</div>
      <div class="sig-line">Klassenlehrer/in</div>
    </div>
  </div>
  {% endfor %}
</body>
</html>
```

### Pipeline in Rust (grob)

```rust
// 1. Daten aus SQLite laden
let daten = db::laden(&auswahl)?;

// 2. Template rendern mit minijinja
let html = template.render(&daten)?;

// 3. PDF via Tauri-WebView2-Print
let pdf_bytes = wry::WebViewBuilder::new()
    .with_html(&html)?
    .print_to_pdf(print_options_landscape_a4())?;

// 4. Speichern und/oder drucken
if auswahl.direkt_drucken {
    print::send_to_default_printer(&pdf_bytes)?;
} else {
    fs::write(&pfad, pdf_bytes)?;
    open::that(&pfad)?;
}
```

### Design-Entscheidungen

1. **WebView2 als PDF-Renderer** — kein headless Chromium extra bundeln. Windows hat WebView2 eh installiert (ab Win10 21H1). Spart ~150 MB Binary. Tauri-Installer installiert WebView2 nach, falls ausnahmsweise nicht da.
2. **Sammel-PDF als Default**, nicht N einzelne Dateien. Ein Workflow, ein Druck-Dialog. Toggle „Einzel-PDFs pro Schüler:in" legt die Dateien stattdessen in `export/<Schuljahr>/<Zeitstempel>/einzeln/<Klasse>_<Nachname>_<Vorname>.pdf` ab.
3. **Kein xls-Export in V1**. Wenn später gewünscht, via `rust_xlsxwriter` nachrüstbar.

### Quality-of-Life

- Live-Preview des ersten Blatts beim Druck-Screen.
- Druck-Historie: `export/SJ2025-26_S2/<Zeitstempel>/` bleibt liegen, für Re-Drucke.

## ASV-BW-Stammdaten-Import

### Recherche-Ergebnis

- **Keine offizielle API** existiert. ZSS/ASD-BW ist für Drittprogramme komplett gesperrt (KISS-Netz + Schul-Credentials).
- ASV-BW nutzt intern **PostgreSQL** (Schema `asv`, System-User `sys` / Default-PW `!!ASV!!`). Technisch zugreifbar, aber off-label und schema-instabil.
- **De-facto-Standard**: XLSX/CSV-Export aus der GUI. So machen's alle (DieSchulApp, Schulmanager Online, WebUntis, linuxmuster, Perpustakaan).
- **Kein XÖV/XSchule** in BW produktiv genutzt.

### Unsere Import-Strategie

1. **Mitgelieferte `.exf`-Exportvorlage** (`import-vorlagen/asv-export-template.exf`). Admin importiert die einmal in ASV-BW → künftig Zwei-Klick-Export mit festen Spalten: `ASV-UUID, Klasse, Nachname, Vorname`.
2. **Tolerant parsender Import**: Heuristisches Spalten-Matching (`nachname|familienname|name`, `vorname|rufname`, `klasse|klassenbezeichnung`, `id|schülerid|uuid`). Wenn die Heuristik keine eindeutige Spalte findet oder mehrere Spalten auf dasselbe Feld matchen, zeigt der Import einen Zuordnungs-Dialog mit Vorschau der ersten 5 Zeilen.
3. **Re-Import via ASV-UUID**: Bestehende Schüler:innen werden gematcht und aktualisiert (Namensänderungen, Klassenwechsel). Neue werden angelegt, fehlende behalten.
4. **Fallback für manuelle Einträge**: Schüler:innen ohne UUID (Mittjahreszugänge, per Hand eingetragen) werden über `(vorname, nachname, klasse)` gematcht.

### Nicht im Scope

- Direkter DB-Zugriff auf `asv`-PostgreSQL.
- ZSS-Integration.

## Migration aus Legacy-Jiraso

### In Scope (einmaliger Wizard beim ersten Start)

| Quelle | Ziel | Mechanismus |
|---|---|---|
| `Config/alleSchueler.xlsx` | `schuljahr`, `klasse`, `schueler` | Import via „Schuljahr anlegen"-Wizard |
| `Config/Fächer.txt` | `fach` (mit `reihenfolge`) | vorausgefüllt aus Datei |
| `Config/Floskeln.txt` + Kategorie-Namen aus `format.xls` | `kategorie` + `formulierung` | Parser, Trenner `-` teilt Kategorien |
| Admin-Passwörter | `config.toml` | neu gesetzt mit Argon2id-Hash |

### Nicht übernommen

- `Config/format.xls` — Template wird als HTML nachgebaut.
- `K/<Klasse>/<Schüler>/*.txt` (alte Bewertungen) — beginnen frisch pro Schuljahr.
- `B/<Klasse>/<Schüler>Dokument.xls` — bleiben als Papier-/Datei-Archiv.
- `Beurteilungen_2015/` bis `Beurteilungen_2022/` (10+ Jahre alte Schülerdaten) — DSGVO-technisch nicht ohne Grund in neues System ziehen.

### Migrations-Wizard-UI

1. „Willkommen. Neues Schuljahr anlegen: Name [2026/27]"
2. „Schüler:innen importieren: `alleSchueler.xlsx` wählen" → Spalten-Zuordnungs-Dialog
3. „Fächer importieren oder tippen"
4. „Kategorien und Formulierungen importieren" (Kategorie-Namen einmal zuordnen)
5. „Passwörter setzen" (drei Felder, mind. 12 Zeichen)
6. Fertig.

## Tests

### Ebenen

- **Rust-Unit-Tests**: Import-Parser (ASV-Varianten, kaputte Spalten, Umlaute UTF-8/Latin-1), Lock-Mechanismus (abgelaufene Sessions, 2-aktiv-blockiert, Admin-Override), Schema-Migrationen. Alles „reine Logik".
- **Integration-Test (Headless-Tauri via `tauri-driver`)**: Happy-Path pro Rolle (Login → Bewertung → Auto-Save → PDF → Valid-PDF-Check). Ein Test pro Rolle.
- **Manueller Smoke-Test vor Release**: `SMOKE_CHECKLIST.md` mit ~15 Punkten, u. a.: 2-Nutzer-Session auf Netzlaufwerk, Batch-Druck 30 Schüler:innen, Re-Import `.exf` mit Namensänderung.

### Explizit nicht

- Frontend-Unit-Tests (Vitest) — zu viel Aufwand für diese Größe, Integration-Tests decken Smoke-Pfade ab.

## Deployment

### Build

- GitHub Actions (`windows-latest`), `tauri build` erzeugt signiertes `Jiraso-reloaded.exe`.
- Code-Signing via selbstgeneriertem Zertifikat, schulintern in Windows-Zertifikate-Root importiert. Kein teures EV-Zertifikat nötig bei schulinterner Verteilung.

### Verteilung

- Release-Zip auf GitHub (privat): `.exe` + `templates/` + `import-vorlagen/` + `config/zitate.toml` + `README.md` + `CHANGELOG.md` + `SMOKE_CHECKLIST.md`.
- Admin entpackt nach `Jiraso-reloaded/` auf dem Netzlaufwerk.

### Updates

- Admin ersetzt `.exe`. DB bleibt, `schema_version`-Tabelle triggert Schema-Migrationen beim ersten Start.
- Kein Auto-Updater in V1.

### Repo-Struktur

```
/
├── src-tauri/          Rust-Backend
│   ├── src/
│   │   ├── main.rs
│   │   ├── db.rs
│   │   ├── import.rs   (xlsx-Parser, ASV-heuristisch)
│   │   ├── lock.rs
│   │   ├── pdf.rs
│   │   └── ...
│   ├── tauri.conf.json
│   └── Cargo.toml
├── src/                Svelte-Frontend
│   ├── routes/
│   ├── lib/
│   └── app.html
├── templates/
│   └── beilagenblatt.html
├── import-vorlagen/
│   └── asv-export-template.exf
├── config/
│   └── zitate.toml
├── docs/
│   └── superpowers/
│       ├── specs/      (diese Datei)
│       └── plans/      (kommt nach writing-plans)
├── .github/workflows/  build.yml, release.yml
├── README.md
└── LICENSE
```

## Goodie: Lehrerinnen-Kachel

Kleine, dezente Kachel oben auf dem Dashboard. Erscheint einmal pro Session beim Login, via `×` für heute wegklickbar. Drei Arten, zufällig rotiert:

1. **Pädagogisches Tageszitat** aus `config/zitate.toml` (~80 kuratierte Zitate von Montessori, Pestalozzi, Hentig, Korczak, Rogers, Rilke, Astrid Lindgren, Hermann Hesse u. a. — nachdenklich, nicht kitschig).
2. **Fun-Fact** (~200 Stück) aus Sprache / Schulgeschichte / Kuriositäten. Nichts Politisches, nichts Kontroverses.
3. **Fortschritts-Feier** (situativ): „🎉 Geschafft! Mathematik 5b — alle 28 Schüler:innen bewertet." oder „🌻 Schönes Wochenende! Du hast 7 Bemerkungen fertig gemacht."

Opt-Out in `config.toml`:

```toml
[goodies]
enabled = true
mode = "random"   # oder "zitat_only" / "fun_fact_only" / "achievement_only"
```

Zitat-Index pro Tag deterministisch (`Datum → Hash`), damit beide parallel arbeitenden Kolleg:innen dasselbe Zitat sehen und sich im Lehrerzimmer austauschen können.

## Offene Punkte / TODOs vor Implementierung

1. **Sample-Export aus echter ASV-BW-Instanz** besorgen (Klassenliste + Schülerstamm), um Import-Parser gegen reale Daten zu testen.
2. **Kategorie-Namen-Mapping** aus `format.xls` abschließend finalisieren (R8, R12, R15–17, R19–21, R22–24, R26–27, R30–31 — teilweise kombiniert über mehrere Zeilen).
3. **Schul-Briefkopf**: sollen wir das aktuelle Logo/Anschrift des Schiller-Gymnasiums im Template hartkodieren oder als konfigurierbare Felder in der `config.toml` halten? → Für V1 konfigurierbar, damit die Schule das selbst anpassen kann.
4. **Goodies-Inhalt** kuratieren — die 80 Zitate und 200 Fun-Facts werden im Implementations-Schritt zusammengestellt.

## Nicht im Scope für V1 (bewusst)

- Persönliche Lehrer-Accounts.
- Audit-Log pro Person.
- Versionierung/History der Bewertungen.
- Realtime-Multi-User-Sync.
- Offline-/Merge-Support.
- Lokaler LLM / KI-Unterstützung bei Bemerkungs-Formulierungen.
- Auto-Updater.
- Mac/Linux-Builds.
- xls-Export.
- Echte ASV-BW-API-Integration.
