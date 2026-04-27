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
