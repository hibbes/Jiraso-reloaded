-- src-tauri/src/migrations/003_bewertung_pk.sql
-- Plan-2-Daten in bewertung sind leer (kein Import-Pfad), deshalb verlustfreier DROP+CREATE.
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
