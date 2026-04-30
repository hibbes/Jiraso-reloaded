-- src-tauri/src/migrations/004_editor_kuerzel.sql
-- Wer hat zuletzt geschrieben? Schutz gegen Überschreiben-fremder-KuK-Bewertungen
-- (typischer Fehler: Lehrer:in tippt im falschen Klassen-Tab und überschreibt).
ALTER TABLE bewertung ADD COLUMN editor_kuerzel TEXT;
ALTER TABLE bemerkung ADD COLUMN editor_kuerzel TEXT;
