<!-- src/routes/setup/+page.svelte -->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { setupPasswoerter } from '$lib/api';

  let fachlehrer = $state('');
  let klassenlehrer = $state('');
  let administrator = $state('');
  let fehler = $state<string | null>(null);
  let laufend = $state(false);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    fehler = null;
    laufend = true;
    try {
      await setupPasswoerter(fachlehrer, klassenlehrer, administrator);
      goto('/login');
    } catch (err) {
      fehler = String(err);
    } finally {
      laufend = false;
    }
  }
</script>

<div class="setup-wrap">
  <div class="card setup-card">
    <h1>Erste Einrichtung</h1>
    <p class="text-muted">
      Bitte lege die drei Rollen-Passwörter fest (jeweils mindestens 8 Zeichen).
      Gib sie danach persönlich an die jeweiligen Kolleg:innen weiter.
    </p>

    <form onsubmit={submit}>
      <label>
        <span class="label-row">Fachlehrer <span class="badge role-fachlehrer">fachlehrer</span></span>
        <input type="password" bind:value={fachlehrer} minlength="8" required />
      </label>
      <label>
        <span class="label-row">Klassenlehrer <span class="badge role-klassenlehrer">klassenlehrer</span></span>
        <input type="password" bind:value={klassenlehrer} minlength="8" required />
      </label>
      <label>
        <span class="label-row">Administrator <span class="badge role-administrator">administrator</span></span>
        <input type="password" bind:value={administrator} minlength="8" required />
      </label>

      <button type="submit" disabled={laufend}>
        {laufend ? 'Speichere …' : 'Speichern'}
      </button>
      {#if fehler}
        <p class="err" role="alert">{fehler}</p>
      {/if}
    </form>
  </div>
</div>

<style>
  .setup-wrap {
    display: flex;
    justify-content: center;
    padding-top: 2.5rem;
  }
  .setup-card {
    width: 100%;
    max-width: 480px;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-top: 1.2rem;
  }
  .label-row {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }
  :global(.badge.role-fachlehrer) {
    background: var(--sg-success);
    color: #fff;
    border-color: transparent;
  }
  :global(.badge.role-klassenlehrer) {
    background: var(--sg-gold);
    color: var(--sg-petrol);
    border-color: transparent;
    font-weight: 600;
  }
  :global(.badge.role-administrator) {
    background: var(--sg-danger);
    color: #fff;
    border-color: transparent;
  }
  .err {
    color: var(--sg-danger);
    margin: 0;
    font-size: 0.88rem;
  }
</style>
