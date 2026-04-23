// src/lib/session.svelte.ts
import type { Rolle } from './api';

class SessionStore {
  rolle = $state<Rolle | null>(null);
  schule = $state<string>('');
  schuljahr = $state<string>('');
}

export const session = new SessionStore();
