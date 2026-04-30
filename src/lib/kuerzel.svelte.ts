// src/lib/kuerzel.svelte.ts
//
// Kürzel der aktuell arbeitenden Lehrkraft. Wird vor dem ersten Bewerten
// abgefragt, in localStorage zwischengespeichert (1 Woche TTL) und an
// jeden Save-Call mitgegeben. Backend persistiert es pro Bewertung +
// Bemerkung als editor_kuerzel — das Frontend zeigt damit Banner+Konflikt-
// Warnungen, wenn fremde Kürzel bereits Einträge in derselben Klasse/Fach
// hinterlegt haben (Schutz gegen "falsche Klasse"-Überschreiben).

const KEY = 'jiraso-kuerzel-v1';
const TTL_MS = 7 * 24 * 60 * 60 * 1000;

type Persisted = { kuerzel: string; gesetzt_am: number };

class KuerzelStore {
  kuerzel = $state<string | null>(null);

  load() {
    if (typeof localStorage === 'undefined') return;
    try {
      const raw = localStorage.getItem(KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as Persisted;
      if (Date.now() - parsed.gesetzt_am < TTL_MS && parsed.kuerzel) {
        this.kuerzel = parsed.kuerzel;
      } else {
        localStorage.removeItem(KEY);
      }
    } catch {
      localStorage.removeItem(KEY);
    }
  }

  set(neu: string) {
    const trimmed = neu.trim().slice(0, 8).toUpperCase();
    if (!trimmed) return;
    this.kuerzel = trimmed;
    if (typeof localStorage !== 'undefined') {
      const data: Persisted = { kuerzel: trimmed, gesetzt_am: Date.now() };
      localStorage.setItem(KEY, JSON.stringify(data));
    }
  }

  clear() {
    this.kuerzel = null;
    if (typeof localStorage !== 'undefined') {
      localStorage.removeItem(KEY);
    }
  }
}

export const kuerzelStore = new KuerzelStore();
