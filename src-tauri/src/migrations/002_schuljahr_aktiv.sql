-- src-tauri/src/migrations/002_schuljahr_aktiv.sql
-- Nur ein Schuljahr darf aktiv=1 haben; aktiv=0 beliebig oft.
CREATE UNIQUE INDEX idx_schuljahr_nur_eins_aktiv
    ON schuljahr(aktiv) WHERE aktiv = 1;
