<!-- src/routes/uebersicht/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    klassenraum,
    type Schuljahr,
    type Klasse,
    type SchuelerMini
  } from '$lib/api';

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let klassen = $state<Klasse[]>([]);
  let aktiveKlasseId = $state<number | null>(null);
  let schueler = $state<SchuelerMini[]>([]);
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (!session.rolle) {
      goto('/login');
      return;
    }
    try {
      const list = await stammdaten.list();
      aktivesSchuljahr = list.find(s => s.aktiv) ?? null;
      if (!aktivesSchuljahr) {
        fehler = 'Kein aktives Schuljahr.';
        return;
      }
      klassen = await klassenraum.klassen(aktivesSchuljahr.id);
      if (klassen.length > 0) {
        aktiveKlasseId = klassen[0].id;
        await ladeSchueler();
      }
    } catch (e) { fehler = String(e); }
  });

  async function ladeSchueler() {
    if (!aktiveKlasseId) { schueler = []; return; }
    schueler = await klassenraum.schueler(aktiveKlasseId);
  }

  $effect(() => { ladeSchueler(); });
</script>

<main class="container">
  <header>
    <h1>Modul-Übersicht</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}
    <p class="error">Fehler: {fehler}</p>
  {/if}

  {#if aktivesSchuljahr}
    <p class="meta">Schuljahr: <strong>{aktivesSchuljahr.bezeichnung}</strong></p>

    <label class="klasse-pick">
      Klasse:
      <select bind:value={aktiveKlasseId}>
        {#each klassen as k (k.id)}
          <option value={k.id}>{k.name}</option>
        {/each}
      </select>
    </label>

    {#if schueler.length === 0}
      <p class="leer">Keine Schüler:innen in dieser Klasse.</p>
    {:else}
      <ul class="schueler-list">
        {#each schueler as s (s.id)}
          <li>
            <a href={`/uebersicht/${s.id}`}>
              <span class="name">{s.nachname}, {s.vorname}</span>
              <span class="pfeil">→</span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</main>

<style>
  .container { max-width: 800px; margin: 0 auto; padding: 2rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .meta { color: #555; }
  .klasse-pick { display: block; margin: 1rem 0 1.5rem; }
  .klasse-pick select { padding: 0.4rem; margin-left: 0.4rem; min-width: 150px; }
  .leer { color: #999; font-style: italic; }
  .schueler-list { list-style: none; padding: 0; }
  .schueler-list li a {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.7rem 1rem;
    border-bottom: 1px solid #ddd;
    text-decoration: none;
    color: inherit;
  }
  .schueler-list li a:hover { background: #f4f6f8; }
  .name { font-weight: 500; }
  .pfeil { color: var(--sg-petrol, #004058); }
</style>
