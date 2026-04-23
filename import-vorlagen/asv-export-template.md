# ASV-BW-Exportvorlage für Jiraso-reloaded

Diese Anleitung beschreibt, wie eine Exportvorlage in ASV-BW angelegt wird, die einen für Jiraso passenden XLSX-Export erzeugt.

## Schritt-für-Schritt in ASV-BW

1. Menü „Auswertungen" → „Individueller Export" öffnen.
2. Export-Typ: **XLSX** wählen.
3. Spalten zufügen (per Drag & Drop):
   - `Schüler.ID` → Kopfzeile umbenennen in `ASV-UUID`
   - `Schüler.Klasse` → Kopfzeile `Klasse`
   - `Schüler.Nachname` → Kopfzeile `Nachname`
   - `Schüler.Vorname` → Kopfzeile `Vorname`
4. Filter: aktuelles Schuljahr, nur Klassen 5 und 6.
5. Vorlage speichern als `jiraso-reloaded-v1.exf`.
6. Die `.exf`-Datei in diesen Ordner kopieren und mit Jiraso-reloaded zusammen auf das Netzlaufwerk legen.

## Verwendung

- Ein Admin importiert einmalig `jiraso-reloaded-v1.exf` in seine ASV-BW-Instanz.
- Danach genügt ein Klick, um den passenden XLSX-Export zu erzeugen.
- In Jiraso-reloaded: Admin → Stammdaten-Import → XLSX wählen.

## Hinweis

Die Spalten-Erkennung in Jiraso-reloaded ist heuristisch und funktioniert auch mit anderen Kopfzeilen (z. B. „Familienname" statt „Nachname", „Rufname" statt „Vorname"). Die genaue Bezeichnung ist also nicht kritisch, solange jede Spalte eindeutig ist.
