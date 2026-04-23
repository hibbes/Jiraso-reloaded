<!-- src/routes/login/+page.svelte -->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { login, rechnerName } from '$lib/api';
  import { session } from '$lib/session.svelte';

  let passwort = $state('');
  let fehler = $state<string | null>(null);
  let laufend = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    fehler = null;
    laufend = true;
    try {
      const rolle = await login(passwort, rechnerName());
      session.rolle = rolle;
      goto('/dashboard');
    } catch (err) {
      fehler = String(err);
    } finally {
      laufend = false;
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
    </form>
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
    max-width: 380px;
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
    color: var(--sg-danger);
    margin: 0.25rem 0 0;
    font-size: 0.88rem;
  }
</style>
