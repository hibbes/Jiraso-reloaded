// src/lib/api.ts
import { invoke } from '@tauri-apps/api/core';

export type Rolle = 'fachlehrer' | 'klassenlehrer' | 'administrator';

export async function login(passwort: string, rechner: string): Promise<Rolle> {
  return invoke<Rolle>('login', { passwort, rechner });
}

export async function logout(): Promise<void> {
  return invoke<void>('logout');
}

export async function currentRole(): Promise<Rolle | null> {
  return invoke<Rolle | null>('current_role');
}

export async function schulname(): Promise<string> {
  return invoke<string>('schulname');
}

export async function aktuellesSchuljahr(): Promise<string> {
  return invoke<string>('aktuelles_schuljahr');
}

export async function breakLockAdmin(adminPasswort: string): Promise<void> {
  return invoke<void>('break_lock_admin', { adminPasswort });
}

export async function needsSetup(): Promise<boolean> {
  return invoke<boolean>('needs_setup');
}

export async function setupPasswoerter(
  fachlehrer: string,
  klassenlehrer: string,
  administrator: string
): Promise<void> {
  return invoke<void>('setup_passwoerter', { fachlehrer, klassenlehrer, administrator });
}

export function rechnerName(): string {
  // Tauri gibt nicht den Hostnamen — wir nutzen den User-Agent-Hash als Pseudo-ID
  // oder, bessere Variante, rufen ein Rust-Command auf, das std::env::var("COMPUTERNAME")
  // liest. Für jetzt: navigator.userAgent-Hash.
  const ua = navigator.userAgent;
  let h = 0;
  for (let i = 0; i < ua.length; i++) h = (h * 31 + ua.charCodeAt(i)) | 0;
  return 'PC-' + (h >>> 0).toString(16).slice(0, 6);
}

// --- Stammdaten / Import ---

export type Schuljahr = {
  id: number;
  bezeichnung: string;
  aktiv: boolean;
};

export type ColumnMapping = {
  uuid: number | null;
  klasse: number;
  nachname: number;
  vorname: number;
};

export type FieldKind = 'Uuid' | 'Klasse' | 'Nachname' | 'Vorname';

export type DetectResult =
  | { Ok: ColumnMapping }
  | {
      Ambiguous: {
        headers: string[];
        suggestions: Record<FieldKind, number[]>;
      };
    };

export type ParsedSheet = {
  headers: string[];
  rows: string[][];
};

export type ImportPreview = {
  sheet: ParsedSheet;
  detection: DetectResult;
};

export type ImportSummary = {
  neue_klassen: number;
  neue_schueler: number;
  geaenderte_schueler: number;
  unveraenderte_schueler: number;
};

export const stammdaten = {
  list: () => invoke<Schuljahr[]>('list_schuljahre'),
  anlegen: (bezeichnung: string, aktivieren: boolean) =>
    invoke<number>('schuljahr_anlegen', { bezeichnung, aktivieren }),
  aktivieren: (id: number) => invoke<void>('schuljahr_aktivieren', { id })
};

export const importStammdaten = {
  preview: (bytes: number[]) =>
    invoke<ImportPreview>('import_preview', { bytes }),
  apply: (schuljahrId: number, bytes: number[], mapping: ColumnMapping) =>
    invoke<ImportSummary>('import_apply', {
      schuljahrId,
      bytes,
      mapping
    })
};

// --- Katalog ---

export type Fach = {
  id: number;
  schuljahr_id: number;
  name: string;
  reihenfolge: number;
  aktiv: boolean;
};

export type Kategorie = {
  id: number;
  schuljahr_id: number;
  name: string;
  reihenfolge: number;
  aktiv: boolean;
};

export type Formulierung = {
  id: number;
  kategorie_id: number;
  text: string;
  reihenfolge: number;
  aktiv: boolean;
};

export const katalog = {
  faecher: (schuljahrId: number) =>
    invoke<Fach[]>('katalog_faecher', { schuljahrId }),
  kategorien: (schuljahrId: number) =>
    invoke<Kategorie[]>('katalog_kategorien', { schuljahrId }),
  formulierungen: (kategorieId: number) =>
    invoke<Formulierung[]>('katalog_formulierungen', { kategorieId }),

  fachAnlegen: (schuljahrId: number, name: string) =>
    invoke<number>('katalog_fach_anlegen', { schuljahrId, name }),
  seedDefaultFaecher: (schuljahrId: number) =>
    invoke<{ neue_faecher: number; uebersprungene_faecher: number }>(
      'katalog_seed_default_faecher',
      { schuljahrId }
    ),
  kategorieAnlegen: (schuljahrId: number, name: string) =>
    invoke<number>('katalog_kategorie_anlegen', { schuljahrId, name }),
  formulierungAnlegen: (kategorieId: number, text: string) =>
    invoke<number>('katalog_formulierung_anlegen', { kategorieId, text }),

  fachAktiv: (id: number, aktiv: boolean) =>
    invoke<void>('katalog_fach_aktiv', { id, aktiv }),
  kategorieAktiv: (id: number, aktiv: boolean) =>
    invoke<void>('katalog_kategorie_aktiv', { id, aktiv }),
  formulierungAktiv: (id: number, aktiv: boolean) =>
    invoke<void>('katalog_formulierung_aktiv', { id, aktiv }),

  fachReihenfolge: (id: number, reihenfolge: number) =>
    invoke<void>('katalog_fach_reihenfolge', { id, reihenfolge }),
  kategorieReihenfolge: (id: number, reihenfolge: number) =>
    invoke<void>('katalog_kategorie_reihenfolge', { id, reihenfolge }),
  formulierungReihenfolge: (id: number, reihenfolge: number) =>
    invoke<void>('katalog_formulierung_reihenfolge', { id, reihenfolge })
};

// --- Bewertung ---

export type MatrixZelle = {
  schueler_id: number;
  kategorie_id: number;
  formulierung_id: number | null;
  geaendert_am: string;
};

export type BewertungUpdate = {
  schueler_id: number;
  fach_id: number;
  kategorie_id: number;
  formulierung_id: number | null;
  vorheriger_stand: string | null;
};

export type SetResult =
  | { status: 'Ok'; neuer_stand: string }
  | { status: 'Konflikt'; server_formulierung_id: number | null; server_geaendert_am: string };

export const bewertung = {
  matrix: (klasseId: number, fachId: number) =>
    invoke<MatrixZelle[]>('bewertung_matrix', { klasseId, fachId }),
  set: (update: BewertungUpdate) =>
    invoke<SetResult>('bewertung_set', { update })
};

// --- Bemerkung ---

export const bemerkung = {
  get: (schuelerId: number) =>
    invoke<[string, string] | null>('bemerkung_get', { schuelerId }),
  set: (schuelerId: number, text: string, vorherigerStand: string | null) =>
    invoke<SetResult>('bemerkung_set', { schuelerId, text, vorherigerStand })
};

// --- Legacy-Import ---

export type LegacyKategorie = {
  name: string;
  formulierungen: string[];
};

export type LegacyImportPreview = {
  faecher: string[];
  kategorien: LegacyKategorie[];
};

export type LegacyImportSummary = {
  neue_faecher: number;
  neue_kategorien: number;
  neue_formulierungen: number;
  uebersprungene_faecher: number;
  uebersprungene_kategorien: number;
  uebersprungene_formulierungen: number;
};

export const legacyImport = {
  preview: (faecherBytes: number[], floskelnBytes: number[], formatBytes: number[]) =>
    invoke<LegacyImportPreview>('legacy_import_preview', { faecherBytes, floskelnBytes, formatBytes }),
  apply: (schuljahrId: number, preview: LegacyImportPreview) =>
    invoke<LegacyImportSummary>('legacy_import_apply', { schuljahrId, preview })
};

// --- Klassen + Schüler (für Bewertungs-Matrix, Backend Task 11) ---

export type Klasse = {
  id: number;
  name: string;
  schuljahr_id: number;
};

export type SchuelerMini = {
  id: number;
  vorname: string;
  nachname: string;
  sortname: string;
};

export const klassenraum = {
  klassen: (schuljahrId: number) =>
    invoke<Klasse[]>('klassenraum_klassen', { schuljahrId }),
  schueler: (klasseId: number) =>
    invoke<SchuelerMini[]>('klassenraum_schueler', { klasseId })
};

// --- Bug-Report ---

export type IssueResponse = { number: number; html_url: string };

export const bugReport = {
  submit: (titel: string, body: string) =>
    invoke<IssueResponse>('bug_report_submit', { titel, body })
};
