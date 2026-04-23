<!-- src/routes/+layout.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { session } from '$lib/session.svelte';
  import { schulname, aktuellesSchuljahr, currentRole, logout } from '$lib/api';
  import { goto } from '$app/navigation';

  onMount(async () => {
    session.schule = await schulname();
    session.schuljahr = await aktuellesSchuljahr();
    session.rolle = await currentRole();
  });

  async function handleLogout() {
    await logout();
    session.rolle = null;
    goto('/login');
  }

  let { children } = $props();
</script>

<header>
  <div class="brand">{session.schule}</div>
  <div class="title">Verbalbeurteilungen 5/6 · SJ {session.schuljahr}</div>
  <div class="auth">
    {#if session.rolle}
      <span class="role role-{session.rolle}">{session.rolle}</span>
      <button onclick={handleLogout}>Abmelden</button>
    {:else}
      <span class="role">nicht angemeldet</span>
    {/if}
  </div>
</header>

<main>
  {@render children()}
</main>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.6rem 1.2rem;
    background: #1f2d3d;
    color: #fff;
    font-family: 'Segoe UI', sans-serif;
  }
  .brand { font-weight: bold; }
  .title { font-size: 0.95rem; opacity: 0.9; }
  .auth { display: flex; gap: 0.8rem; align-items: center; }
  .role { font-size: 0.9rem; padding: 0.15rem 0.5rem; border-radius: 3px; background: #39475a; }
  .role-administrator { background: #8b2942; }
  .role-klassenlehrer { background: #1e4d8b; }
  .role-fachlehrer    { background: #2a7a4a; }
  button { padding: 0.25rem 0.8rem; cursor: pointer; }
  main { padding: 1.5rem 2rem; }
</style>
