<!-- src/routes/druck/+page.svelte -->
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
    type SchuelerUebersicht,
    type ModulZelle
  } from '$lib/api';

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let klassen = $state<Klasse[]>([]);
  let aktiveKlasseId = $state<number | null>(null);
  let uebersichten = $state<SchuelerUebersicht[]>([]);
  let auswahl = $state<Record<number, boolean>>({});
  let nurFertige = $state(false);
  let geladen = $state(false);
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (!session.rolle) { goto('/login'); return; }
    try {
      const list = await stammdaten.list();
      aktivesSchuljahr = list.find(s => s.aktiv) ?? null;
      if (!aktivesSchuljahr) { fehler = 'Kein aktives Schuljahr.'; return; }
      klassen = await klassenraum.klassen(aktivesSchuljahr.id);
      if (klassen.length > 0) {
        aktiveKlasseId = klassen[0].id;
      }
    } catch (e) { fehler = String(e); }
  });

  async function ladeUebersichten() {
    if (!aktiveKlasseId) return;
    geladen = false;
    fehler = null;
    try {
      uebersichten = await uebersichtApi.klasse(aktiveKlasseId);
      auswahl = {};
      for (const u of uebersichten) auswahl[u.schueler_id] = true;
      geladen = true;
    } catch (e) { fehler = String(e); }
  }

  $effect(() => { if (aktiveKlasseId) ladeUebersichten(); });

  function gruppen(module: ModulZelle[]) {
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

  const sichtbar = $derived(uebersichten.filter(u => {
    if (!auswahl[u.schueler_id]) return false;
    if (nurFertige && u.bewertete_module < u.gesamt_module) return false;
    return true;
  }));

  function alle() { for (const u of uebersichten) auswahl[u.schueler_id] = true; }
  function keine() { for (const u of uebersichten) auswahl[u.schueler_id] = false; }
  function nurFertigeAuswaehlen() {
    for (const u of uebersichten) auswahl[u.schueler_id] = u.bewertete_module === u.gesamt_module;
  }
  function drucken() { window.print(); }
</script>

<main class="container no-print">
  <header>
    <h1>Drucken &amp; Export</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}
    <p class="error">Fehler: {fehler}</p>
  {/if}

  {#if aktivesSchuljahr}
    <p class="meta">Schuljahr <strong>{aktivesSchuljahr.bezeichnung}</strong></p>

    <div class="controls">
      <label>Klasse:
        <select bind:value={aktiveKlasseId}>
          {#each klassen as k (k.id)}
            <option value={k.id}>{k.name}</option>
          {/each}
        </select>
      </label>

      <label class="toggle">
        <input type="checkbox" bind:checked={nurFertige} />
        Nur fertige anzeigen
      </label>
    </div>

    {#if geladen && uebersichten.length > 0}
      <section class="auswahl">
        <div class="auswahl-actions">
          <button onclick={alle}>Alle</button>
          <button onclick={keine}>Keine</button>
          <button onclick={nurFertigeAuswaehlen}>Nur Fertige</button>
        </div>
        <ul class="schueler-grid">
          {#each uebersichten as u (u.schueler_id)}
            <li class:fertig={u.bewertete_module === u.gesamt_module}>
              <label>
                <input type="checkbox" bind:checked={auswahl[u.schueler_id]} />
                <span class="name">{u.nachname}, {u.vorname}</span>
                <span class="status">{u.bewertete_module}/{u.gesamt_module}</span>
              </label>
            </li>
          {/each}
        </ul>
      </section>

      <div class="druck-row">
        <p class="zaehler">
          {sichtbar.length} von {uebersichten.length} Schüler:innen ausgewählt
        </p>
        <button class="druck" onclick={drucken} disabled={sichtbar.length === 0}>
          🖨️ Drucken / als PDF speichern
        </button>
        <p class="pdf-hinweis">
          Im Druck-Dialog „Microsoft Print to PDF" oder „Als PDF speichern" auswählen.
        </p>
      </div>

      <section class="vorschau">
        <h2>Vorschau</h2>
        <p class="vorschau-hinweis">So sieht jeder ausgedruckte Bogen aus (gekürzte Anzeige):</p>
      </section>
    {:else if geladen && uebersichten.length === 0}
      <p class="leer">Keine Schüler:innen in dieser Klasse.</p>
    {/if}
  {/if}
</main>

<!-- Druck-Bereich: jede Schülerin/jeder Schüler eine A4-Seite -->
<div class="druck-bereich">
  {#each sichtbar as u (u.schueler_id)}
    <article class="zeugnis-seite">
      <div class="kopf">
        <h2>{u.vorname} {u.nachname}</h2>
        <p class="kopf-meta">
          Klasse <strong>{u.klasse_name}</strong>
          · Schuljahr <strong>{u.schuljahr_bezeichnung}</strong>
        </p>
      </div>

      {#each gruppen(u.module) as g (g.fach_id)}
        <section class="fach">
          <h3>{g.fach_name}</h3>
          <dl>
            {#each g.zellen as z (`${z.fach_id}-${z.kategorie_id}`)}
              <dt>{z.kategorie_name}</dt>
              <dd class:keine-angabe={z.bewertet && !z.formulierung_text} class:offen={!z.bewertet}>
                {#if z.formulierung_text}
                  {z.formulierung_text}
                {:else if z.bewertet}
                  — keine Angabe —
                {:else}
                  ····················
                {/if}
              </dd>
            {/each}
          </dl>
        </section>
      {/each}

      {#if u.bemerkung}
        <section class="bemerkung">
          <h3>Bemerkung</h3>
          <p>{u.bemerkung}</p>
        </section>
      {/if}
    </article>
  {/each}
</div>

<style>
  .container { max-width: 900px; margin: 0 auto; padding: 1.5rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .meta { color: #555; }
  .controls { display: flex; gap: 1.2rem; align-items: center; flex-wrap: wrap; margin: 1rem 0 1.5rem; }
  .controls select { padding: 0.4rem; min-width: 150px; margin-left: 0.4rem; }
  .toggle { display: flex; align-items: center; gap: 0.4rem; }

  .auswahl { background: #f7f8fa; padding: 1rem; border-radius: 6px; }
  .auswahl-actions { display: flex; gap: 0.5rem; margin-bottom: 0.6rem; }
  .auswahl-actions button { padding: 0.3rem 0.7rem; font-size: 0.85rem; }
  .schueler-grid { list-style: none; padding: 0; margin: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 0.3rem; }
  .schueler-grid li label { display: flex; gap: 0.4rem; align-items: center; padding: 0.3rem; cursor: pointer; }
  .schueler-grid li.fertig label { color: #060; }
  .schueler-grid li label:hover { background: #fff; }
  .schueler-grid .name { flex: 1; }
  .schueler-grid .status { color: #888; font-size: 0.85rem; font-variant-numeric: tabular-nums; }

  .druck-row {
    display: flex;
    gap: 1rem;
    align-items: center;
    flex-wrap: wrap;
    margin: 1.5rem 0;
    padding: 1rem;
    background: #fff8de;
    border: 1px solid #e0c060;
    border-radius: 6px;
  }
  .druck { padding: 0.6rem 1.2rem; font-size: 1rem; background: var(--sg-petrol, #004058); color: white; border: 0; border-radius: 6px; cursor: pointer; }
  .druck:hover { background: #00567a; }
  .druck:disabled { opacity: 0.5; cursor: not-allowed; }
  .zaehler { margin: 0; color: #555; }
  .pdf-hinweis { margin: 0; flex-basis: 100%; font-size: 0.85rem; color: #666; }

  .vorschau h2 { margin-top: 2rem; }
  .vorschau-hinweis { color: #666; font-size: 0.9rem; }

  .druck-bereich { display: none; }
  .leer { color: #999; font-style: italic; }

  /* Vorschau-Modus: zeige die ersten paar zeugnis-seiten klein gestapelt */
  @media screen {
    .druck-bereich {
      display: block;
      max-width: 760px;
      margin: 0 auto;
      padding-bottom: 4rem;
    }
    .zeugnis-seite {
      background: white;
      box-shadow: 0 2px 12px rgba(0,0,0,0.1);
      padding: 2rem 2.4rem;
      margin-bottom: 1.5rem;
      border-radius: 4px;
    }
  }

  .kopf {
    border-bottom: 2px solid #333;
    padding-bottom: 0.6rem;
    margin-bottom: 1.2rem;
  }
  .kopf h2 { margin: 0; }
  .kopf-meta { color: #444; margin: 0.3rem 0 0; font-size: 0.95rem; }

  .fach { margin-bottom: 1.4rem; break-inside: avoid; }
  .fach h3 {
    margin: 0 0 0.3rem;
    font-size: 1.05rem;
    border-bottom: 1px solid #999;
    padding-bottom: 0.15rem;
  }
  dl { margin: 0; display: grid; grid-template-columns: 12rem 1fr; gap: 0.25rem 1rem; }
  dt { font-weight: 600; color: #333; }
  dd { margin: 0; }
  dd.keine-angabe { color: #777; font-style: italic; }
  dd.offen { color: #aaa; letter-spacing: 0.1em; }

  .bemerkung { margin-top: 1.4rem; padding-top: 0.6rem; border-top: 1px solid #999; break-inside: avoid; }
  .bemerkung h3 {
    margin: 0 0 0.3rem;
    font-size: 1.05rem;
  }
  .bemerkung p { margin: 0; white-space: pre-wrap; }

  @media print {
    @page { size: A4 portrait; margin: 1.6cm 1.8cm; }
    .no-print { display: none !important; }
    body { background: white; color: black; }
    .druck-bereich { display: block; }
    .zeugnis-seite {
      page-break-after: always;
      box-shadow: none;
      padding: 0;
      margin: 0;
      border-radius: 0;
    }
    .zeugnis-seite:last-child { page-break-after: auto; }
    dt { color: black; }
    .fach h3, .bemerkung h3, .kopf { border-color: black; }
  }
</style>
