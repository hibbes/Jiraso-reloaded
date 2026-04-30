# Jiraso-reloaded

> **Status: Re-Implementation / "Reloaded-Fork" des ursprünglichen Jiraso.**
> Dies ist *kein* Fork der 2021er-Codebase im git-Sinne (anderer Tech-Stack),
> sondern eine konzeptionelle Neuauflage, die Modell und Workflow von Peer
> Schliekers Original übernimmt.

Modernisierte Verbalbeurteilungs-Software fuer die Klassen 5 und 6 am
Schiller-Gymnasium Offenburg.

**Initiator und Ideengeber:** Peer Schlieker hat 2021 das ursprüngliche Jiraso
entwickelt und damit das Konzept der modulbasierten Verbalbeurteilung am
Schiller-Gymnasium etabliert. *Jiraso-reloaded* übernimmt sein didaktisches
Modell und seinen Workflow-Ansatz und baut beides auf einem neuen Tech-Stack
neu auf, damit das Werkzeug langfristig wartbar bleibt. Dank an Peer für die
Idee, das Pionierwerk und die Bereitschaft, das Konzept weitergeben zu lassen.

**Tech-Stack:** Tauri 2 (Rust) + SvelteKit 2 (Svelte 5, TypeScript) + SQLite (bundled).

## Features

- **Foundation** (Plan 1): Passwort-Setup, Rollen-Login (Fachlehrer/Klassenlehrer/Admin), Session-Lock auf Netzlaufwerk, SQLite mit Tages-Backup.
- **Stammdaten-Import** (ab Plan 2, Admin-only): XLSX-Import aus ASV-BW mit heuristischer Spalten-Erkennung, ASV-UUID-basiertem Re-Import, Schuljahr-Verwaltung.

## Installation auf dem Schulrechner (Plan 1: Foundation)

1. Lade den aktuellen Windows-Build aus den
   [GitHub Releases](https://github.com/hibbes/Jiraso-reloaded/releases)
   oder als Artefakt der neuesten
   [Windows-Build-Action](https://github.com/hibbes/Jiraso-reloaded/actions/workflows/build.yml).
2. Entpacke den Inhalt nach `V:\Jiraso-reloaded\` auf dem Tauschlaufwerk
   (oder jedem anderen gemeinsamen Pfad).
3. Starte `Jiraso-reloaded.exe` per Doppelklick. Beim ersten Start:
   - Die Setup-Seite erscheint.
   - Lege drei Passwörter fest (Fachlehrer, Klassenlehrer, Admin,
     jeweils mindestens 8 Zeichen).
   - Nach dem Speichern kannst du dich einloggen.
4. Teste die Rollen (Fachlehrer sieht weniger Kacheln als Admin).
5. Starte die App parallel auf einem zweiten Rechner — der dritte
   gleichzeitige Start soll mit Hinweis auf die beiden aktiven Sessions
   abgelehnt werden.

Neben der `.exe` entstehen automatisch:

- `config.toml` — Schule, Schuljahr, Passwort-Hashes, Lock-Limits
- `data/jiraso.db` — SQLite-Datenbank (WAL-Modus)
- `data/jiraso.db.lock` — JSON-Liste der aktiven Sessions
- `data/backups/jiraso-YYYY-MM-DD.db` — täglicher Snapshot, rotiert auf 30 Tage

Eine vollständige Abnahme-Liste fuer die Schul-Installation liegt unter
[`SMOKE_CHECKLIST.md`](SMOKE_CHECKLIST.md).

## Stammdaten importieren

1. Als **Admin** einloggen.
2. Dashboard → „Stammdaten-Import".
3. Schuljahr wählen oder neu anlegen (Format `YYYY/YY`, z. B. `2025/26`).
4. XLSX aus ASV-BW wählen (siehe [`import-vorlagen/asv-export-template.md`](import-vorlagen/asv-export-template.md)).
5. Vorschau prüfen, ggf. Spalten zuordnen, bestätigen.

## Entwicklung

Voraussetzungen:

- Node.js 20+ und npm
- Rust 1.80+ (stable)
- Linux-Entwicklung: `webkit2gtk-4.1`, `libsoup-3.0`, `gtk+-3.0` (Tauri-Prerequisites)
- Windows-Entwicklung: Visual Studio Build Tools + WebView2 (vorinstalliert auf Win10+)

```bash
npm install
npm run tauri dev   # Debug-Build mit Hot-Reload
```

### Tests (Rust-Backend)

Das Backend kann ohne Tauri-Runtime und ohne webkit2gtk getestet werden:

```bash
cd src-tauri
cargo test --no-default-features --lib
```

17 Unit-Tests decken DB-Migration, Config, Auth (Argon2id), Lock und
Backup ab.

### Frontend-Check

```bash
npm run check   # svelte-kit sync + svelte-check
npm run build   # adapter-static build
```

## Release-Build

Der Windows-Release-Build laeuft auf jedem Push nach `main` (oder bei
Tag `v*`) automatisch via GitHub Actions:
[`.github/workflows/build.yml`](.github/workflows/build.yml).

Das Artefakt `Jiraso-reloaded-Windows` enthaelt:

- `jiraso-reloaded.exe` (portable Binary)
- NSIS-Installer (`*.exe` unter `bundle/nsis/`)
- MSI-Installer (`*.msi` unter `bundle/msi/`)

Fuer einen Release mit GitHub-Release-Entry:

```bash
git tag v0.1.0
git push --tags
```

Die Action erstellt dann automatisch eine GitHub-Release mit den beiden
Installern als Assets.

## Projekt-Struktur

```
Jiraso-reloaded/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Entry
│   │   ├── lib.rs               # run()-Setup + Feature-Gate
│   │   ├── error.rs             # AppError (thiserror, serialisierbar)
│   │   ├── db.rs                # SQLite + Migrations-Runner
│   │   ├── config.rs            # config.toml laden/erzeugen
│   │   ├── auth.rs              # Argon2id hash/verify, Rollen
│   │   ├── lock.rs              # Session-Lock auf dem Netzlaufwerk
│   │   ├── backup.rs            # Taeglicher DB-Snapshot
│   │   ├── commands.rs          # #[tauri::command]-Wrapper
│   │   └── migrations/
│   │       └── 001_initial.sql
│   └── Cargo.toml
├── src/                         # SvelteKit
│   ├── routes/
│   │   ├── +layout.svelte
│   │   ├── +layout.ts           # ssr=false, prerender=false
│   │   ├── +page.svelte         # Root-Redirect
│   │   ├── login/+page.svelte
│   │   ├── setup/+page.svelte
│   │   └── dashboard/+page.svelte
│   └── lib/
│       ├── api.ts               # Typed invoke-Wrapper
│       └── session.svelte.ts    # Svelte-5-Rune-Store
├── .github/workflows/build.yml  # Windows-Build
├── config.toml.example
└── docs/superpowers/            # Spec + Plans
```

## Nächste Schritte (Folge-Pläne)

Plan 1 (dieses Repository) stellt das Grundgerüst bereit. Kommende
Arbeitspakete:

- **Plan 2** — Stammdaten-Import (Klassen, Schüler aus ASV-Export)
- **Plan 3** — Bewertungs- und Bemerkungs-UI mit optimistischer
  Konfliktprüfung
- **Plan 4** — Übersicht + Admin-Editor fuer Formulierungen
- **Plan 5** — PDF-Druck im vorgegebenen Layout
- **Plan 6** — Goodies, i18n und Release-Politur

## Lizenz

[MIT License](./LICENSE) — frei nutzbar, modifizierbar, weiter­verteilbar.

**Ausnahme:** `static/schiller_logo.png` ist Eigentum des
Schiller-Gymnasiums Offenburg, alle Rechte vorbehalten. Forks für
andere Schulen müssen das Logo durch ihr eigenes ersetzen, bevor
weiterverteilt wird.

Fragen, Bug-Reports und Pull-Requests willkommen über das
[GitHub-Issue-Tracker](https://github.com/hibbes/Jiraso-reloaded/issues).
