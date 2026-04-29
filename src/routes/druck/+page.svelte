<!-- src/routes/druck/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    klassenraum,
    katalog,
    uebersicht as uebersichtApi,
    type Schuljahr,
    type Klasse,
    type Fach,
    type Kategorie,
    type Formulierung,
    type SchuelerUebersicht
  } from '$lib/api';

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let klassen = $state<Klasse[]>([]);
  let aktiveKlasseId = $state<number | null>(null);
  let faecher = $state<Fach[]>([]);
  let kategorien = $state<Kategorie[]>([]);
  let formulierungenByKat = $state<Record<number, Formulierung[]>>({});
  let uebersichten = $state<SchuelerUebersicht[]>([]);
  let auswahl = $state<Record<number, boolean>>({});
  let nurFertige = $state(false);
  let geladen = $state(false);
  let fehler = $state<string | null>(null);
  let datumStr = $state(new Date().toLocaleDateString('de-DE'));

  onMount(async () => {
    if (!session.rolle) { goto('/login'); return; }
    try {
      const list = await stammdaten.list();
      aktivesSchuljahr = list.find(s => s.aktiv) ?? null;
      if (!aktivesSchuljahr) { fehler = 'Kein aktives Schuljahr.'; return; }
      klassen = await klassenraum.klassen(aktivesSchuljahr.id);
      faecher = (await katalog.faecher(aktivesSchuljahr.id)).filter(f => f.aktiv);
      kategorien = (await katalog.kategorien(aktivesSchuljahr.id)).filter(k => k.aktiv);
      for (const k of kategorien) {
        formulierungenByKat[k.id] = (await katalog.formulierungen(k.id)).filter(f => f.aktiv);
      }
      if (klassen.length > 0) aktiveKlasseId = klassen[0].id;
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

  // Set aller (formulierung_id × fach_id)-Paare, die für einen Schüler gewählt sind
  function gewaehlt(u: SchuelerUebersicht): Set<string> {
    const set = new Set<string>();
    for (const m of u.module) {
      if (m.formulierung_id != null) {
        set.add(`${m.formulierung_id}:${m.fach_id}`);
      }
    }
    return set;
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

  const aktiveKlasse = $derived(klassen.find(k => k.id === aktiveKlasseId) ?? null);
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
      <label>Datum:
        <input type="text" bind:value={datumStr} placeholder="29.04.2026" />
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
        <p class="zaehler">{sichtbar.length} von {uebersichten.length} ausgewählt</p>
        <button class="druck" onclick={drucken} disabled={sichtbar.length === 0}>
          🖨️ Drucken / als PDF speichern
        </button>
        <p class="pdf-hinweis">
          Im Druck-Dialog „Microsoft Print to PDF" oder „Als PDF speichern" auswählen.
          Eine Seite pro Schüler:in.
        </p>
      </div>
    {:else if geladen && uebersichten.length === 0}
      <p class="leer">Keine Schüler:innen in dieser Klasse.</p>
    {/if}
  {/if}
</main>

<!-- Druck-Bereich: jede Schülerin/jeder Schüler eine A4-Seite im Original-Layout -->
<div class="druck-bereich">
  {#each sichtbar as u (u.schueler_id)}
    {@const auswahlSet = gewaehlt(u)}
    <article class="bogen">
      <header class="bogen-kopf">
        <div class="schule">
          <img src="/schiller_logo.png" alt="Schiller-Gymnasium Offenburg" class="logo" />
        </div>
        <div class="titel">
          <div class="titel-haupt">Allgemeine Beurteilung</div>
          <div class="titel-name">{u.vorname} {u.nachname}</div>
        </div>
        <div class="schuljahr-klasse">
          Schuljahr {u.schuljahr_bezeichnung}<br />
          Klasse: {u.klasse_name}
        </div>
      </header>

      <table class="matrix">
        <thead>
          <tr>
            <th class="kat-spalte"></th>
            <th class="form-spalte"></th>
            {#each faecher as f (f.id)}
              <th class="fach-spalte"><span class="fach-name">{f.name}</span></th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each kategorien as k (k.id)}
            {@const forms = formulierungenByKat[k.id] ?? []}
            {#each forms as form, i (form.id)}
              <tr>
                {#if i === 0}
                  <th class="kat-name" rowspan={forms.length}>{k.name}</th>
                {/if}
                <td class="form-text">{form.text}</td>
                {#each faecher as f (f.id)}
                  <td class="kreuz">
                    {#if auswahlSet.has(`${form.id}:${f.id}`)}X{/if}
                  </td>
                {/each}
              </tr>
            {/each}
          {/each}
        </tbody>
      </table>

      <section class="bemerkung">
        <div class="bem-titel">zusätzliche Bemerkungen:</div>
        <div class="bem-text">{u.bemerkung ?? ''}</div>
      </section>

      <footer class="bogen-fuss">
        <div class="datum-zeile">
          <span>Offenburg, den {datumStr}</span>
          <span class="unterschrift-linie">&nbsp;</span>
          <span class="unterschrift-linie">&nbsp;</span>
        </div>
        <div class="rolle-zeile">
          <span></span>
          <span class="rolle">Klassenlehrer/in</span>
          <span class="rolle">Gesehen! Erziehungsberechtigte/r</span>
        </div>
      </footer>
    </article>
  {/each}
</div>

<style>
  /* Kontroll-UI */
  .container { max-width: 900px; margin: 0 auto; padding: 1.5rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .meta { color: #555; }
  .controls { display: flex; gap: 1.2rem; align-items: center; flex-wrap: wrap; margin: 1rem 0 1.5rem; }
  .controls select, .controls input[type="text"] { padding: 0.4rem; margin-left: 0.4rem; }
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
    display: flex; gap: 1rem; align-items: center; flex-wrap: wrap;
    margin: 1.5rem 0; padding: 1rem; background: #fff8de;
    border: 1px solid #e0c060; border-radius: 6px;
  }
  .druck { padding: 0.6rem 1.2rem; font-size: 1rem; background: var(--sg-petrol, #004058); color: white; border: 0; border-radius: 6px; cursor: pointer; }
  .druck:hover { background: #00567a; }
  .druck:disabled { opacity: 0.5; cursor: not-allowed; }
  .zaehler { margin: 0; color: #555; }
  .pdf-hinweis { margin: 0; flex-basis: 100%; font-size: 0.85rem; color: #666; }

  .leer { color: #999; font-style: italic; }

  /* Bogen — Original-Matrix-Layout (Floskel × Fach) */
  .druck-bereich { display: block; max-width: 770px; margin: 0 auto; padding-bottom: 4rem; }

  @media screen {
    .bogen {
      background: white;
      box-shadow: 0 2px 12px rgba(0,0,0,0.1);
      padding: 1.4rem 1.6rem;
      margin-bottom: 1.5rem;
      border-radius: 4px;
    }
  }

  .bogen-kopf {
    display: grid;
    grid-template-columns: 1fr 2fr 1fr;
    align-items: start;
    gap: 0.8rem;
    margin-bottom: 0.6rem;
  }
  .schule { display: flex; align-items: center; }
  .schule .logo { height: 60px; width: auto; }
  .titel { text-align: center; }
  .titel-haupt { font-weight: 600; font-size: 1rem; margin-bottom: 0.3rem; }
  .titel-name { font-size: 1.05rem; font-weight: 500; border-bottom: 1px solid #000; padding-bottom: 0.15rem; min-width: 12rem; display: inline-block; }
  .schuljahr-klasse { font-size: 0.9rem; line-height: 1.4; text-align: right; }

  .matrix {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.78rem;
    table-layout: fixed;
    box-sizing: border-box;
    border: 1px solid #444;
  }
  .matrix th, .matrix td {
    border: 1px solid #444;
    padding: 0.15rem 0.3rem;
    box-sizing: border-box;
    word-break: break-word;
  }
  .matrix thead tr { height: 5.4rem; }
  .matrix .kat-spalte { width: 17%; border: 0; }
  .matrix .form-spalte { width: 35%; border: 0; }
  .matrix .fach-spalte {
    width: 4%;
    padding: 0;
    vertical-align: bottom;
    border-bottom: 1px solid #444;
    border-top: 1px solid #444;
  }
  .matrix .fach-name {
    writing-mode: vertical-rl;
    transform: rotate(180deg);
    display: inline-block;
    padding: 0.3rem 0;
    font-size: 0.8rem;
    font-weight: 500;
    white-space: nowrap;
  }
  .matrix .kat-name {
    background: #f0f0f0;
    font-weight: 600;
    text-align: left;
    vertical-align: middle;
    font-size: 0.72rem;
    line-height: 1.15;
    /* Wortbrueche nur an Wortgrenzen, nicht mitten im Wort */
    word-break: normal;
    overflow-wrap: normal;
    hyphens: none;
    padding: 0.2rem 0.3rem;
  }
  .matrix .form-text {
    text-align: left;
    line-height: 1.2;
  }
  .matrix .kreuz {
    text-align: center;
    font-weight: 600;
    width: 4%;
    padding: 0;
  }

  .bemerkung {
    margin-top: 0.6rem;
    border: 1px solid #444;
    padding: 0.3rem 0.5rem;
  }
  .bem-titel { font-size: 0.8rem; color: #333; margin-bottom: 0.2rem; }
  .bem-text {
    min-height: 7rem;
    white-space: pre-wrap;
    font-size: 0.85rem;
    line-height: 1.35;
  }

  .bogen-fuss {
    margin-top: 0.8rem;
    font-size: 0.8rem;
  }
  .datum-zeile { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 0.6rem; align-items: end; }
  .unterschrift-linie {
    border-bottom: 1px solid #444;
    height: 1.1rem;
  }
  .rolle-zeile {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 0.6rem;
    margin-top: 0.15rem;
    color: #555;
    font-size: 0.72rem;
  }
  .rolle { text-align: center; }

  @media print {
    @page { size: A4 portrait; margin: 1.3cm; }
    html, body { margin: 0 !important; padding: 0 !important; }
    .no-print { display: none !important; }
    body { background: white; color: black; }
    .druck-bereich { max-width: none; padding: 0; margin: 0; }
    .bogen {
      page-break-after: always;
      box-shadow: none;
      padding: 0;
      margin: 0;
      border-radius: 0;
      /* Verfuegbare Hoehe = 29.7cm - 2 * 1.3cm = 27.1cm — Reserve einplanen */
      max-height: 27cm;
      overflow: hidden;
    }
    .bogen:last-child { page-break-after: auto; }
    .bogen-kopf { margin-bottom: 0.4rem; }
    .schule .logo { height: 50px; }
    .titel-haupt { font-size: 0.95rem; margin-bottom: 0.2rem; }
    .titel-name { font-size: 1rem; }
    .schuljahr-klasse { font-size: 0.82rem; }

    .matrix { font-size: 8.5pt; }
    .matrix th, .matrix td { padding: 0.08rem 0.25rem; }
    .matrix thead tr { height: 4.8rem; }
    .matrix .fach-name { font-size: 8.5pt; padding: 0.2rem 0; }
    .matrix .kat-name {
      background: #eee !important;
      -webkit-print-color-adjust: exact;
      print-color-adjust: exact;
      font-size: 8.5pt;
      padding: 0.15rem 0.3rem;
    }
    .matrix .form-text { font-size: 8.5pt; line-height: 1.15; }

    .bemerkung { margin-top: 0.4rem; padding: 0.25rem 0.4rem; }
    .bem-titel { font-size: 8.5pt; margin-bottom: 0.15rem; }
    .bem-text { font-size: 9pt; line-height: 1.3; min-height: 4rem; max-height: 5.5rem; overflow: hidden; }

    .bogen-fuss { margin-top: 1rem; font-size: 9pt; }
    .datum-zeile { gap: 0.8rem; }
    .unterschrift-linie { height: 1.5rem; }
    .rolle-zeile { font-size: 8pt; margin-top: 0.25rem; gap: 0.8rem; }
  }
</style>
