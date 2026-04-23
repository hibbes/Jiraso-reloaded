-- src-tauri/src/migrations/001_initial.sql
CREATE TABLE schuljahr (
    id          INTEGER PRIMARY KEY,
    bezeichnung TEXT NOT NULL UNIQUE,
    aktiv       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE klasse (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    UNIQUE(schuljahr_id, name)
);

CREATE TABLE schueler (
    id          INTEGER PRIMARY KEY,
    asv_uuid    TEXT UNIQUE,
    klasse_id   INTEGER NOT NULL REFERENCES klasse(id) ON DELETE CASCADE,
    vorname     TEXT NOT NULL,
    nachname    TEXT NOT NULL,
    sortname    TEXT NOT NULL GENERATED ALWAYS AS (nachname || ', ' || vorname) STORED
);
CREATE INDEX idx_schueler_klasse ON schueler(klasse_id, nachname);

CREATE TABLE fach (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    reihenfolge  INTEGER NOT NULL,
    aktiv        INTEGER NOT NULL DEFAULT 1,
    UNIQUE(schuljahr_id, name)
);

CREATE TABLE kategorie (
    id           INTEGER PRIMARY KEY,
    schuljahr_id INTEGER NOT NULL REFERENCES schuljahr(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    reihenfolge  INTEGER NOT NULL,
    aktiv        INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE formulierung (
    id           INTEGER PRIMARY KEY,
    kategorie_id INTEGER NOT NULL REFERENCES kategorie(id) ON DELETE CASCADE,
    text         TEXT NOT NULL,
    reihenfolge  INTEGER NOT NULL,
    aktiv        INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE bewertung (
    schueler_id     INTEGER NOT NULL REFERENCES schueler(id) ON DELETE CASCADE,
    fach_id         INTEGER NOT NULL REFERENCES fach(id) ON DELETE CASCADE,
    formulierung_id INTEGER NOT NULL REFERENCES formulierung(id) ON DELETE CASCADE,
    geaendert_am    TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (schueler_id, fach_id, formulierung_id)
);

CREATE TABLE bemerkung (
    schueler_id   INTEGER PRIMARY KEY REFERENCES schueler(id) ON DELETE CASCADE,
    text          TEXT NOT NULL,
    geaendert_am  TEXT NOT NULL DEFAULT (datetime('now'))
);
