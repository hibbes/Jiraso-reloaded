<!-- src/routes/setup/+page.svelte -->
<script lang="ts">
  import { goto } from '$app/navigation';
  import { setupPasswoerter } from '$lib/api';

  let fachlehrer = $state('');
  let klassenlehrer = $state('');
  let administrator = $state('');
  let fehler = $state<string | null>(null);

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    fehler = null;
    try {
      await setupPasswoerter(fachlehrer, klassenlehrer, administrator);
      goto('/login');
    } catch (err) {
      fehler = String(err);
    }
  }
</script>

<h1>Erste Einrichtung</h1>
<p>Bitte lege die drei Rollen-Passwörter fest (jeweils mindestens 8 Zeichen).
Gib sie danach persönlich an die jeweiligen Kolleg:innen weiter.</p>

<form onsubmit={submit}>
  <label>Fachlehrer    <input type="password" bind:value={fachlehrer}    minlength="8" required /></label>
  <label>Klassenlehrer <input type="password" bind:value={klassenlehrer} minlength="8" required /></label>
  <label>Administrator <input type="password" bind:value={administrator} minlength="8" required /></label>
  <button type="submit">Speichern</button>
  {#if fehler}<p class="err">{fehler}</p>{/if}
</form>

<style>
  form { display: flex; flex-direction: column; gap: 0.8rem; max-width: 360px; }
  label { display: flex; flex-direction: column; gap: 0.3rem; }
  input { padding: 0.5rem; }
  button { padding: 0.6rem; cursor: pointer; }
  .err { color: #b00; }
</style>
