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
