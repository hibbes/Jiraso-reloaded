<!-- src/routes/uebersicht/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    klassenraum,
    uebersicht as uebersichtApi,
    type Schuljahr,
    type Klasse,
    type SchuelerMini,
    type VollstReport,
    type VollstFach
  } from '$lib/api';

  type Tab = 'schueler' | 'vollstaendigkeit';

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let klassen = $state<Klasse[]>([]);
  let aktiveKlasseId = $state<number | null>(null);
  let schueler = $state<SchuelerMini[]>([]);
  let vollst = $state<VollstReport | null>(null);
  let tab = $state<Tab>('schueler');
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
      }
    } catch (e) { fehler = String(e); }
  });

  async function ladeKlassenDaten() {
    if (!aktiveKlasseId) { schueler = []; vollst = null; return; }
    try {
      schueler = await klassenraum.schueler(aktiveKlasseId);
      vollst = await uebersichtApi.vollstaendigkeit(aktiveKlasseId);
    } catch (e) { fehler = String(e); }
  }

  $effect(() => { ladeKlassenDaten(); });

  function fachKlasse(f: VollstFach): string {
    if (f.gesamt === 0) return '';
    if (f.bewertet === 0) return 'leer';
    if (f.bewertet === f.gesamt) return 'voll';
    return 'teil';
  }
  function katKlasse(bewertet: number, gesamt: number): string {
    if (gesamt === 0) return '';
    if (bewertet === 0) return 'leer';
    if (bewertet === gesamt) return 'voll';
    return 'teil';
  }
  function pct(b: number, g: number): number {
    return g === 0 ? 0 : Math.round((b / g) * 100);
  }
  function bewertenSpringen() {
    if (aktiveKlasseId) goto(`/bewertung?klasse=${aktiveKlasseId}`);
  }
</script>

<main class="container">
  <header>
    <h1>Übersicht</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}
    <p class="error">Fehler: {fehler}</p>
  {/if}

  {#if aktivesSchuljahr}
    <p class="meta">Schuljahr: <strong>{aktivesSchuljahr.bezeichnung}</strong></p>

    <div class="controls">
      <label>Klasse:
        <select bind:value={aktiveKlasseId}>
          {#each klassen as k (k.id)}
            <option value={k.id}>{k.name}</option>
          {/each}
        </select>
      </label>

      <div class="tabs" role="tablist" aria-label="Sicht waehlen">
        <button
          role="tab"
          aria-selected={tab === 'schueler'}
          class:aktiv={tab === 'schueler'}
          onclick={() => tab = 'schueler'}
        >Pro Schüler:in</button>
        <button
          role="tab"
          aria-selected={tab === 'vollstaendigkeit'}
          class:aktiv={tab === 'vollstaendigkeit'}
          onclick={() => tab = 'vollstaendigkeit'}
        >Vollständigkeit (Klasse × Fach)</button>
      </div>
    </div>

    {#if tab === 'schueler'}
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
    {:else if vollst}
      {#if vollst.schueler_anzahl === 0}
        <p class="leer">Keine Schüler:innen in dieser Klasse.</p>
      {:else if vollst.faecher.length === 0}
        <p class="leer">Keine aktiven Fächer im Schuljahr.</p>
      {:else}
        <div class="vollst-summary">
          <strong>{vollst.schueler_anzahl}</strong> Schüler:innen ·
          <strong>{vollst.bewertet_gesamt}</strong> / {vollst.gesamt_gesamt} Bewertungen
          ({pct(vollst.bewertet_gesamt, vollst.gesamt_gesamt)}%)
          <button class="bewerten-btn" onclick={bewertenSpringen}>📝 Zur Bewertung</button>
        </div>

        <div class="vollst-table-wrap">
          <table class="vollst-table">
            <thead>
              <tr>
                <th class="fach-col">Fach</th>
                {#each vollst.faecher[0].kategorien as k (k.kategorie_id)}
                  <th>{k.kategorie_name}</th>
                {/each}
                <th class="summe-col">Σ</th>
              </tr>
            </thead>
            <tbody>
              {#each vollst.faecher as f (f.fach_id)}
                <tr class={fachKlasse(f)}>
                  <th class="fach-col">{f.fach_name}</th>
                  {#each f.kategorien as k (k.kategorie_id)}
                    <td class={katKlasse(k.bewertet, k.gesamt)}>
                      <span class="zahl">{k.bewertet}/{k.gesamt}</span>
                    </td>
                  {/each}
                  <td class="summe-col {fachKlasse(f)}">
                    <span class="zahl">{f.bewertet}/{f.gesamt}</span>
                    <span class="pct">{pct(f.bewertet, f.gesamt)}%</span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        <div class="legende">
          <span class="lg-zelle voll">voll</span>
          <span class="lg-zelle teil">angefangen</span>
          <span class="lg-zelle leer">leer</span>
        </div>
      {/if}
    {/if}
  {/if}
</main>

<style>
  .container { max-width: 1100px; margin: 0 auto; padding: 2rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .meta { color: var(--sg-meta, #666); }

  .controls {
    display: flex;
    gap: 1.5rem;
    align-items: center;
    margin: 1rem 0 1.5rem;
    flex-wrap: wrap;
  }
  .controls select {
    padding: 0.4rem;
    margin-left: 0.4rem;
    min-width: 150px;
  }

  .tabs {
    display: inline-flex;
    border: 1px solid var(--sg-border, #cfd6dd);
    border-radius: 6px;
    overflow: hidden;
    margin-left: auto;
  }
  .tabs button {
    border: 0;
    background: var(--sg-surface, #fafafa);
    color: var(--sg-text, #333);
    padding: 0.4rem 1rem;
    cursor: pointer;
    font-size: 0.9rem;
  }
  .tabs button.aktiv {
    background: var(--sg-petrol, #004058);
    color: white;
  }
  .tabs button:not(.aktiv):hover { background: var(--sg-bg-card, #eee); }

  .leer { color: #999; font-style: italic; }

  /* --- Tab "Pro Schueler:in" --- */
  .schueler-list { list-style: none; padding: 0; }
  .schueler-list li a {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.7rem 1rem;
    border-bottom: 1px solid var(--sg-border, #ddd);
    text-decoration: none;
    color: inherit;
  }
  .schueler-list li a:hover { background: var(--sg-bg-card, #f4f6f8); }
  .schueler-list .name { font-weight: 500; }
  .schueler-list .pfeil { color: var(--sg-petrol, #004058); }

  /* --- Tab "Vollstaendigkeit" --- */
  .vollst-summary {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.7rem 1rem;
    background: var(--sg-bg-card, #f4f6f8);
    border: 1px solid var(--sg-border, #ddd);
    border-radius: 6px;
    margin-bottom: 1rem;
    font-size: 0.95rem;
    flex-wrap: wrap;
  }
  .bewerten-btn {
    margin-left: auto;
    padding: 0.4rem 0.9rem;
    background: var(--sg-petrol, #004058);
    color: white;
    border: 0;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.88rem;
  }
  .bewerten-btn:hover { background: #00567a; }

  .vollst-table-wrap { overflow-x: auto; }
  .vollst-table {
    border-collapse: collapse;
    width: 100%;
    font-size: 0.9rem;
  }
  .vollst-table th,
  .vollst-table td {
    border: 1px solid var(--sg-border, #cfd6dd);
    padding: 0.5rem 0.7rem;
    text-align: center;
    vertical-align: middle;
  }
  .vollst-table thead th {
    background: var(--sg-bg-card, #f4f6f8);
    color: var(--sg-petrol, #004058);
    font-size: 0.85rem;
    font-weight: 600;
  }
  .vollst-table tbody th.fach-col {
    text-align: left;
    background: var(--sg-bg-card, #f4f6f8);
    font-weight: 500;
    min-width: 8rem;
  }
  .vollst-table .summe-col {
    background: #fafafa;
    font-weight: 600;
  }
  .vollst-table td.voll,
  .vollst-table .summe-col.voll {
    background: #e1f4dc;
    color: #0a4015;
  }
  .vollst-table td.teil,
  .vollst-table .summe-col.teil {
    background: #fef4d6;
    color: #6b4a00;
  }
  .vollst-table td.leer,
  .vollst-table .summe-col.leer {
    background: #fdecec;
    color: #7a1a1a;
  }
  .vollst-table .zahl { font-variant-numeric: tabular-nums; display: block; }
  .vollst-table .summe-col .pct {
    display: block;
    font-size: 0.78rem;
    opacity: 0.7;
    margin-top: 0.1rem;
  }

  .legende {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.8rem;
    font-size: 0.82rem;
  }
  .lg-zelle {
    padding: 0.15rem 0.6rem;
    border-radius: 3px;
    border: 1px solid var(--sg-border, #cfd6dd);
  }
  .lg-zelle.voll { background: #e1f4dc; color: #0a4015; }
  .lg-zelle.teil { background: #fef4d6; color: #6b4a00; }
  .lg-zelle.leer { background: #fdecec; color: #7a1a1a; }
</style>
