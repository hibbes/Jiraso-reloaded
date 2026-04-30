<!-- src/routes/uebersicht/[schueler_id]/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { session } from '$lib/session.svelte';
  import { uebersicht, type SchuelerUebersicht, type ModulZelle } from '$lib/api';

  let daten = $state<SchuelerUebersicht | null>(null);
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (!session.rolle) {
      goto('/login');
      return;
    }
    const id = Number(page.params.schueler_id);
    if (!Number.isFinite(id)) {
      fehler = 'Ungültige Schüler-ID.';
      return;
    }
    try {
      daten = await uebersicht.schueler(id);
    } catch (e) { fehler = String(e); }
  });

  function gruppen(module: ModulZelle[]): Array<{ fach_id: number; fach_name: string; zellen: ModulZelle[] }> {
    const result: Array<{ fach_id: number; fach_name: string; zellen: ModulZelle[] }> = [];
    for (const m of module) {
      let g = result.find(r => r.fach_id === m.fach_id);
      if (!g) {
        g = { fach_id: m.fach_id, fach_name: m.fach_name, zellen: [] };
        result.push(g);
      }
      g.zellen.push(m);
    }
    return result;
  }
</script>

<main class="container">
  <header class="no-print">
    <h1>Modul-Übersicht</h1>
    <nav>
      <a href="/uebersicht">← Schülerliste</a>
      <a href="/dashboard">Dashboard</a>
    </nav>
  </header>

  {#if fehler}
    <p class="error">Fehler: {fehler}</p>
  {/if}

  {#if daten}
    <article class="zeugnis">
      <div class="kopf">
        <h2>{daten.vorname} {daten.nachname}</h2>
        <p class="meta">
          Klasse <strong>{daten.klasse_name}</strong>
          · Schuljahr <strong>{daten.schuljahr_bezeichnung}</strong>
        </p>
        <p class="fortschritt no-print">
          Bewertet: <strong>{daten.bewertete_module} / {daten.gesamt_module}</strong> Module
          {#if daten.bewertete_module === daten.gesamt_module && daten.gesamt_module > 0}
            ✓
          {/if}
        </p>
      </div>

      {#each gruppen(daten.module) as g (g.fach_id)}
        <section class="fach">
          <h3>{g.fach_name}</h3>
          <dl>
            {#each g.zellen as z (`${z.fach_id}-${z.kategorie_id}`)}
              <dt>{z.kategorie_name}</dt>
              <dd class:leer={!z.bewertet} class:keine-angabe={z.bewertet && !z.formulierung_text}>
                {#if z.formulierung_text}
                  {z.formulierung_text}
                {:else if z.bewertet}
                  — keine Angabe —
                {:else}
                  <em>noch nicht bewertet</em>
                {/if}
              </dd>
            {/each}
          </dl>
        </section>
      {/each}

      {#if daten.bemerkung}
        <section class="bemerkung">
          <h3>Bemerkung</h3>
          <p>{daten.bemerkung}</p>
        </section>
      {/if}
    </article>
  {/if}
</main>

<style>
  .container { max-width: 850px; margin: 0 auto; padding: 2rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1.5rem; }
  header nav { display: flex; gap: 1rem; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .zeugnis { background: white; padding: 1.5rem 2rem; }
  .kopf { border-bottom: 2px solid var(--sg-petrol, #004058); padding-bottom: 0.8rem; margin-bottom: 1.2rem; }
  .kopf h2 { margin: 0; color: var(--sg-petrol, #004058); }
  .meta { color: #555; margin: 0.3rem 0 0; }
  .fortschritt { margin: 0.4rem 0 0; color: #060; font-size: 0.95rem; }
  .fach { margin-bottom: 1.6rem; break-inside: avoid; }
  .fach h3 {
    margin: 0 0 0.4rem;
    color: var(--sg-petrol, #004058);
    border-bottom: 1px solid #ccc;
    padding-bottom: 0.2rem;
  }
  dl { margin: 0; display: grid; grid-template-columns: max-content 1fr; gap: 0.3rem 1rem; }
  dt { font-weight: 600; color: #333; }
  dd { margin: 0; color: #111; }
  dd.leer { color: #999; }
  dd.keine-angabe { color: #777; font-style: italic; }
  .bemerkung { margin-top: 2rem; padding-top: 1rem; border-top: 2px solid #ddd; break-inside: avoid; }
  .bemerkung h3 { margin: 0 0 0.4rem; color: var(--sg-petrol, #004058); }
  .bemerkung p { margin: 0; white-space: pre-wrap; }

  @media print {
    .no-print { display: none !important; }
    .container { padding: 0; max-width: none; }
    .zeugnis { padding: 0; }
    body { background: white; }
  }
</style>
