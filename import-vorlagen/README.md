# Import-Vorlagen

In diesem Ordner liegen Vorlagen, die Admin-User einmalig in externe Systeme einspielen können, damit Exporte direkt im für Jiraso-reloaded passenden Format erfolgen.

## `asv-export-template.exf`

**Status: Platzhalter** — die echte `.exf`-Datei muss vom Admin in ASV-BW erzeugt und hier abgelegt werden (siehe `asv-export-template.md`).

Ziel-Export-Spalten (in dieser Reihenfolge empfohlen, Reihenfolge ist aber egal — Jiraso erkennt die Spalten heuristisch):

| Spalte | Bedeutung |
|---|---|
| `ASV-UUID` | 36-stellige eindeutige Schüler-ID, bleibt über Schuljahre stabil |
| `Klasse` | z. B. „5a", „6c" |
| `Nachname` | Familienname |
| `Vorname` | Rufname |

Format: XLSX. Header-Zeile oben. Eine Zeile pro Schüler:in.
