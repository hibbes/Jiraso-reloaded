<!-- src/routes/login/+page.svelte -->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { login, rechnerName, breakLockAdmin } from '$lib/api';
  import { session } from '$lib/session.svelte';

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

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    fehler = null;
    entsperrInfo = null;
    laufend = true;
    try {
      const rolle = await login(passwort, rechnerName());
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

  async function lockEntsperren(e: SubmitEvent) {
    e.preventDefault();
    entsperrInfo = null;
    fehler = null;
    entsperrLaufend = true;
    try {
      await breakLockAdmin(adminPw);
      entsperrInfo = 'Lock zurückgesetzt. Bitte jetzt mit deinem Rollen-Passwort einloggen.';
      lockBesetzt = false;
      entsperrModus = false;
      adminPw = '';
    } catch (err) {
      fehler = String(err);
    } finally {
      entsperrLaufend = false;
    }
  }
</script>

<div class="login-wrap">
  <div class="card login-card">
    <h1>Anmelden</h1>
    <p class="hint text-muted">
      Bitte Rollen-Passwort eingeben (Fachlehrer, Klassenlehrer, Administrator).
    </p>
    <form onsubmit={submit}>
      <label>
        Passwort
        <input
          type="password"
          bind:value={passwort}
          autocomplete="current-password"
          required
        />
      </label>
      <button type="submit" disabled={laufend || !passwort}>
        {laufend ? 'Prüfe …' : 'Einloggen'}
      </button>
      {#if fehler}
        <p class="err" role="alert">{fehler}</p>
      {/if}
      {#if entsperrInfo}
        <p class="info" role="status">{entsperrInfo}</p>
      {/if}
    </form>

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
          Falls noch jemand wirklich am Arbeiten ist, kann es sein, dass
          dessen Änderungen verloren gehen.
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
</div>

<style>
  .login-wrap {
    display: flex;
    justify-content: center;
    padding-top: 3.5rem;
  }
  .login-card {
    width: 100%;
    max-width: 420px;
  }
  .hint {
    margin: 0 0 1.2rem 0;
    font-size: 0.88rem;
  }
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
