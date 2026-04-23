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

<div class="login">
  <h1>Anmelden</h1>
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
    <button type="submit" disabled={laufend}>
      {laufend ? 'Prüfe…' : 'Einloggen'}
    </button>
    {#if fehler}<p class="err">{fehler}</p>{/if}
  </form>
</div>

<style>
  .login { max-width: 320px; margin: 4rem auto; font-family: 'Segoe UI', sans-serif; }
  h1 { margin-bottom: 1rem; }
  form { display: flex; flex-direction: column; gap: 0.8rem; }
  label { display: flex; flex-direction: column; gap: 0.3rem; }
  input { padding: 0.5rem; font-size: 1rem; }
  button { padding: 0.6rem; font-size: 1rem; cursor: pointer; }
  .err { color: #b00; margin-top: 0.5rem; }
</style>
