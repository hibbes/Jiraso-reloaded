<!-- src/routes/login/+page.svelte — Rollen-Auswahl + optionales Passwort -->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { login, loginFachlehrer, rechnerName, breakLockAdmin } from '$lib/api';
  import type { Rolle } from '$lib/api';
  import { session } from '$lib/session.svelte';
  import { kuerzelStore } from '$lib/kuerzel.svelte';
  import { onMount } from 'svelte';

  // Sicherheitsnetz: falls die App ohne Logout geschlossen wurde (Crash,
  // x-Knopf oben rechts), liegt das Kuerzel der vorigen Lehrkraft noch in
  // localStorage. Beim erneuten Login-Aufruf wegwerfen, damit niemand
  // versehentlich unter fremdem Kuerzel speichert.
  let idleHinweis = $state(false);
  onMount(() => {
    kuerzelStore.clear();
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search);
      if (params.get('grund') === 'idle') idleHinweis = true;
    }
  });

  type RolleId = 'fachlehrer' | 'klassenlehrer' | 'administrator';

  let auswahl = $state<RolleId | null>(null);
  let passwort = $state('');
  let fehler = $state<string | null>(null);
  let laufend = $state(false);
  let lockBesetzt = $state(false);
  let entsperrModus = $state(false);
  let adminPw = $state('');
  let entsperrLaufend = $state(false);
  let entsperrInfo = $state<string | null>(null);

  function lookLikeLockError(msg: string): boolean {
    const e = msg.toLowerCase();
    return (
      e.includes('locked') ||
      e.includes('belegt') ||
      e.includes('parallel') ||
      e.includes('zwei') ||
      e.includes('aktive sessions') ||
      e.includes('lock')
    );
  }

  async function fachlehrerEinloggen() {
    fehler = null;
    laufend = true;
    try {
      const rolle = await loginFachlehrer(rechnerName());
      session.rolle = rolle;
      goto('/dashboard');
    } catch (err) {
      const msg = String(err);
      fehler = msg;
      lockBesetzt = lookLikeLockError(msg);
    } finally {
      laufend = false;
    }
  }

  async function rolleAuswaehlen(r: RolleId) {
    if (r === 'fachlehrer') {
      await fachlehrerEinloggen();
      return;
    }
    auswahl = r;
    passwort = '';
    fehler = null;
    setTimeout(() => {
      const el = document.getElementById('pw-input');
      if (el instanceof HTMLInputElement) el.focus();
    }, 0);
  }

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    fehler = null;
    laufend = true;
    try {
      const rolle = await login(passwort, rechnerName());
      // Server entscheidet via Passwort, welche Rolle es ist; Auswahl ist nur Hinweis.
      // Falls Server-Rolle nicht zur Auswahl passt: trotzdem rein, aber ggf. warnen.
      session.rolle = rolle;
      if (auswahl && rolle !== (auswahl as Rolle)) {
        // Passwort gehört zu einer anderen Rolle — kein Drama, einloggen lassen.
      }
      goto('/dashboard');
    } catch (err) {
      const msg = String(err);
      fehler = msg;
      lockBesetzt = lookLikeLockError(msg);
    } finally {
      laufend = false;
    }
  }

  async function lockEntsperren(e: SubmitEvent) {
    e.preventDefault();
    entsperrInfo = null;
    fehler = null;
    entsperrLaufend = true;
    try {
      await breakLockAdmin(adminPw);
      entsperrInfo = 'Lock zurückgesetzt. Bitte jetzt erneut anmelden.';
      lockBesetzt = false;
      entsperrModus = false;
      adminPw = '';
    } catch (err) {
      fehler = String(err);
    } finally {
      entsperrLaufend = false;
    }
  }

  function abbrechen() {
    auswahl = null;
    passwort = '';
    fehler = null;
  }
</script>

<div class="login-wrap">
  {#if idleHinweis}
    <div class="idle-hinweis" role="alert">
      Du wurdest nach 10min ohne Aktivitaet automatisch abgemeldet, damit
      der Lock-Slot fuer die naechste Lehrkraft frei ist.
    </div>
  {/if}
  {#if !auswahl}
    <div class="rollen-auswahl">
      <h1>Wer arbeitet hier?</h1>
      <p class="hint text-muted">
        Der Rechner selbst ist passwortgeschützt; die Rolle bestimmt, was du
        in der App tun kannst.
      </p>

      <div class="rollen-grid">
        <button
          type="button"
          class="rollen-karte rolle-fachlehrer"
          onclick={() => rolleAuswaehlen('fachlehrer')}
          disabled={laufend}
        >
          <div class="rk-titel">Fachlehrkraft</div>
          <div class="rk-untertitel">kein Passwort</div>
          <div class="rk-text">
            Kreuze für deine Fächer eintragen.
          </div>
        </button>

        <button
          type="button"
          class="rollen-karte rolle-klassenlehrer"
          onclick={() => rolleAuswaehlen('klassenlehrer')}
          disabled={laufend}
        >
          <div class="rk-titel">Klassenlehrkraft</div>
          <div class="rk-untertitel">Passwort</div>
          <div class="rk-text">
            Bemerkungen schreiben, Bögen drucken.
          </div>
        </button>

        <button
          type="button"
          class="rollen-karte rolle-admin"
          onclick={() => rolleAuswaehlen('administrator')}
          disabled={laufend}
        >
          <div class="rk-titel">Admin</div>
          <div class="rk-untertitel">Passwort</div>
          <div class="rk-text">
            Klassen, Schüler:innen, CSV-Import.
          </div>
        </button>
      </div>

      {#if fehler}
        <p class="err" role="alert">{fehler}</p>
      {/if}
      {#if entsperrInfo}
        <p class="info" role="status">{entsperrInfo}</p>
      {/if}

      {#if lockBesetzt && !entsperrModus}
        <div class="lock-hilfe">
          <p>
            Bereits zwei Sitzungen aktiv. Wenn du sicher bist, dass keine andere
            Sitzung mehr offen ist, kann ein Admin den Lock zurücksetzen.
          </p>
          <button type="button" onclick={() => entsperrModus = true}>
            Lock zurücksetzen (Admin) …
          </button>
        </div>
      {/if}

      {#if entsperrModus}
        <form class="lock-form" onsubmit={lockEntsperren}>
          <h2>Lock zurücksetzen</h2>
          <p class="hint text-muted">
            Mit dem Admin-Passwort werden alle aktiven Sitzungen verworfen.
          </p>
          <label>
            Admin-Passwort
            <input
              type="password"
              bind:value={adminPw}
              autocomplete="current-password"
              required
            />
          </label>
          <div class="actions">
            <button type="button" onclick={() => { entsperrModus = false; adminPw = ''; fehler = null; }}>
              Abbrechen
            </button>
            <button type="submit" disabled={entsperrLaufend || !adminPw}>
              {entsperrLaufend ? 'Setze zurück …' : 'Zurücksetzen'}
            </button>
          </div>
        </form>
      {/if}
    </div>
  {:else}
    <div class="card login-card">
      <h1>{auswahl === 'klassenlehrer' ? 'Klassenlehrkraft' : 'Admin'}</h1>
      <p class="hint text-muted">Bitte Rollen-Passwort eingeben.</p>
      <form onsubmit={submit}>
        <label>
          Passwort
          <input
            id="pw-input"
            type="password"
            bind:value={passwort}
            autocomplete="current-password"
            required
          />
        </label>
        <div class="actions">
          <button type="button" onclick={abbrechen}>Zurück</button>
          <button type="submit" disabled={laufend || !passwort}>
            {laufend ? 'Prüfe …' : 'Einloggen'}
          </button>
        </div>
        {#if fehler}
          <p class="err" role="alert">{fehler}</p>
        {/if}
      </form>
    </div>
  {/if}
</div>

<style>
  .login-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 2.5rem;
  }
  .idle-hinweis {
    background: #fff8de;
    color: #6b4a00;
    border: 1px solid #d8a000;
    border-radius: 6px;
    padding: 0.7rem 1rem;
    font-size: 0.92rem;
    margin-bottom: 1.4rem;
    max-width: 480px;
  }
  .rollen-auswahl {
    width: 100%;
    max-width: 920px;
  }
  .rollen-auswahl h1 {
    margin: 0 0 0.4rem;
  }
  .login-card {
    width: 100%;
    max-width: 420px;
  }
  .hint {
    margin: 0 0 1.4rem 0;
    font-size: 0.92rem;
  }
  .rollen-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  .rollen-karte {
    text-align: left;
    padding: 1.4rem 1.2rem 1.6rem;
    border: 1px solid var(--sg-border, #cfd6dd);
    border-radius: 10px;
    background: var(--sg-surface, #fff);
    color: var(--sg-text, #1a1d22);
    cursor: pointer;
    transition: transform 0.08s ease, box-shadow 0.12s ease, border-color 0.12s ease;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    min-height: 8rem;
  }
  .rollen-karte:hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.08);
    border-color: var(--sg-petrol, #2a7ea1);
  }
  .rollen-karte:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .rk-titel {
    font-size: 1.25rem;
    font-weight: 600;
  }
  .rk-untertitel {
    font-size: 0.78rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--sg-text-muted, #6b727a);
  }
  .rk-text {
    margin-top: 0.4rem;
    font-size: 0.92rem;
    color: var(--sg-text, #1a1d22);
    opacity: 0.85;
  }
  .rolle-fachlehrer { border-left: 4px solid #2a7ea1; }
  .rolle-klassenlehrer { border-left: 4px solid #6a9c3f; }
  .rolle-admin { border-left: 4px solid #b6772e; }

  form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }
  .err {
    color: var(--sg-danger, #c00);
    margin: 0.25rem 0 0;
    font-size: 0.88rem;
  }
  .info {
    color: #060;
    background: #efffe6;
    padding: 0.4rem 0.6rem;
    border-radius: 4px;
    margin: 0.25rem 0 0;
    font-size: 0.88rem;
  }
  .lock-hilfe {
    margin-top: 1.2rem;
    padding: 0.8rem 1rem;
    background: #fff7e0;
    border: 1px solid #e0c060;
    border-radius: 6px;
    font-size: 0.88rem;
  }
  .lock-hilfe p { margin: 0 0 0.6rem; }
  .lock-form {
    margin-top: 1rem;
    padding: 1rem;
    border: 1px solid #ccc;
    border-radius: 6px;
    background: #fafafa;
  }
  .lock-form h2 { margin: 0 0 0.4rem; font-size: 1rem; }
  .actions { display: flex; gap: 0.5rem; justify-content: flex-end; }
</style>
