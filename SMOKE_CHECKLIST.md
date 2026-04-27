# Smoke-Checklist Plan 1: Foundation

Am Schulrechner durchzugehen, bevor Plan 2 gestartet wird.

## Erst-Installation

- [ ] `Jiraso-reloaded.exe` vom Netzlaufwerk startet ohne Fehlermeldung
- [ ] Neben der .exe entsteht `config.toml`, `data/jiraso.db`, `data/backups/`
- [ ] Setup-Seite erscheint mit drei Passwort-Feldern
- [ ] Passwörter unter 8 Zeichen werden abgelehnt
- [ ] Nach Speichern wird zur Login-Seite weitergeleitet

## Login & Rollen

- [ ] Admin-Passwort → Dashboard zeigt 6 Kacheln
- [ ] Klassenlehrer-Passwort → Dashboard zeigt 4 Kacheln (ohne „Formulierungen verwalten" und „Datenverwaltung")
- [ ] Fachlehrer-Passwort → Dashboard zeigt 1 Kachel („Bewertung eingeben")
- [ ] Falsches Passwort → Fehlermeldung auf der Login-Seite
- [ ] Abmelden-Button zurück zur Login-Seite

## Lock-Verhalten

- [ ] Auf Rechner B gleichzeitig einloggen → funktioniert
- [ ] Auf Rechner C einloggen (während A+B aktiv) → Fehler mit Namen der beiden aktiven Rechner
- [ ] Auf Rechner A abmelden → Slot frei, C kann sich einloggen
- [ ] App auf A hart schließen (Task-Manager) → Lock bleibt stehen
- [ ] Nach 8 Stunden oder per manuellem Datei-Löschen von `jiraso.db.lock` wird der stale Slot freigegeben

## Backup

- [ ] Nach erstem Start liegt `data/backups/jiraso-YYYY-MM-DD.db`
- [ ] App am nächsten Tag nochmal starten → neue Backup-Datei mit neuem Datum

## Datenbank

- [ ] `data/jiraso.db` mit SQLite-Browser öffnen → 8 Tabellen vorhanden
  (`schuljahr`, `klasse`, `schueler`, `fach`, `kategorie`, `formulierung`,
  `bewertung`, `bemerkung`)
- [ ] Alle Tabellen sind leer

## Plan 3 Smoke (Bewertung + Bemerkung + Käfer + Sternenregen)

### Vorbereitung
- [ ] Aktives Schuljahr ist gesetzt + mindestens eine Klasse mit ≥2 SuS importiert (Plan 2).
- [ ] Login als Admin.

### Legacy-Import
- [ ] `/admin/legacy-import` öffnen.
- [ ] `Fächer.txt` + `Floskeln.txt` + `format.xls` aus dem Original-Programmpaket auswählen.
- [ ] Vorschau zeigt 12 Fächer + 7 Kategorien (Lernbereitschaft, Auffassungsgabe, Beteiligung am Unterricht, Selbstständigkeit und Kreativität, Sorgfalt, Einhalten von Regeln, Soziales Verhalten) mit je 3-5 Formulierungen.
- [ ] Importieren → Summary zeigt 12/7/26 neue Einträge.
- [ ] Zweiter Import → 0 neu, alle übersprungen (idempotent).

### Katalog-Editor
- [ ] `/admin/katalog` öffnen.
- [ ] Tab Fächer → ein Fach deaktivieren, Reload, deaktiviertes Fach wird grau und durchgestrichen.
- [ ] Tab Kategorien → eine Kategorie ↑↓ verschieben, Reihenfolge aktualisiert.
- [ ] Tab Formulierungen → Kategorie wählen, eine neue Formulierung anlegen.

### Bewertung
- [ ] Logout, Login als Klassenlehrer.
- [ ] `/bewertung` → Klasse + Fach wählen.
- [ ] Matrix erscheint, alle Zellen `— keine Angabe —`.
- [ ] Eine Zelle ändern → ⟳ → ✓ binnen Sekunde.
- [ ] Auf eine Schüler-Zeile klicken → rechtes Panel zeigt Bemerkungs-Textarea.
- [ ] In Textarea tippen → 1.5s nach letztem Tastendruck → ✓.
- [ ] Counter `N/700 Zeichen` wird rot ab 700.

### Konflikt-UX
- [ ] Zwei App-Sessions parallel öffnen.
- [ ] Beide auf gleiche (Schüler, Fach, Kategorie)-Zelle klicken, unterschiedliche Werte wählen.
- [ ] Zweite Session zeigt Konflikt-Banner mit `[Meine] [Ihre]`-Buttons.
- [ ] Beide Pfade testen.

### Fachlehrer-Rolle
- [ ] Logout, Login als Fachlehrer.
- [ ] `/bewertung` → Bemerkungs-Panel zeigt Hinweis „nur für Klassenlehrer:in / Admin".
- [ ] `/admin/katalog` → Redirect auf Dashboard.
- [ ] `/admin/legacy-import` → Redirect auf Dashboard.

### Käfer-Knopf + Sternenregen
- [ ] `[bug_report] github_token = "..."` in `data/config.toml` eintragen (PAT mit `issues:write`-Scope).
- [ ] Käfer-Knopf 🪲 unten rechts ist auf jeder Route sichtbar.
- [ ] Klick → Modal öffnet sich, Titel + Beschreibung eintragen → Senden.
- [ ] Toast „Issue #N angelegt" + Sternenregen erscheint.
- [ ] Issue auf GitHub geprüft.
- [ ] Token leer setzen → erneuter Versuch zeigt Mailto-Fallback-Link.
- [ ] In `/bewertung`: alle Zellen einer kleinen Test-Klasse (z. B. 2 SuS × 7 Kategorien = 14 Zellen) auf irgendetwas setzen (auch „keine Angabe"). Beim Setzen der letzten Zelle: Sternenregen.

### Fenster-Verhalten
- [ ] App startet im Default 1600×1000, zentriert, NICHT maximiert (Rahmen ist sichtbar).
- [ ] Resize via Maus-Drag funktioniert.
