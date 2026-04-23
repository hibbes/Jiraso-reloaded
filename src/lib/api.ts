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
