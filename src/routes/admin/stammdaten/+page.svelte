<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    importXlsx,
    type Schuljahr,
    type ColumnMapping,
    type ImportPreview,
    type ImportSummary
  } from '$lib/api';

  let schuljahre = $state<Schuljahr[]>([]);
  let ausgewaehltesSchuljahr = $state<number | null>(null);
  let neueBezeichnung = $state('');
  let schritt = $state<'auswahl' | 'datei' | 'preview' | 'fertig'>('auswahl');
  let preview = $state<ImportPreview | null>(null);
  let mapping = $state<ColumnMapping | null>(null);
  let bytesBuffer = $state<number[] | null>(null);
  let summary = $state<ImportSummary | null>(null);
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (session.rolle !== 'administrator') {
      goto('/dashboard');
      return;
    }
    await refresh();
  });

  async function refresh() {
    schuljahre = await stammdaten.list();
  }

  async function neuesSchuljahr() {
    fehler = null;
    if (!/^\d{4}\/\d{2}$/.test(neueBezeichnung.trim())) {
      fehler = 'Format: YYYY/YY (z.B. 2025/26)';
      return;
    }
    try {
      const id = await stammdaten.anlegen(neueBezeichnung.trim(), true);
      neueBezeichnung = '';
      await refresh();
      ausgewaehltesSchuljahr = id;
      schritt = 'datei';
    } catch (e) {
      fehler = String(e);
    }
  }

  function weiterZuDatei() {
    if (ausgewaehltesSchuljahr === null) {
      fehler = 'Bitte zuerst ein Schuljahr wählen oder anlegen.';
      return;
    }
    fehler = null;
    schritt = 'datei';
  }

  async function dateiGewaehlt(e: Event) {
    const input = e.target as HTMLInputElement;
    const f = input.files?.[0];
    if (!f) return;
    fehler = null;
    try {
      const buf = new Uint8Array(await f.arrayBuffer());
      bytesBuffer = Array.from(buf);
      preview = await importXlsx.preview(bytesBuffer);
      if ('Ok' in preview.detection) {
        mapping = preview.detection.Ok;
      } else {
        // Initialer Mapping-Vorschlag aus den Suggestions (nimm jeweils ersten Kandidaten)
        const s = preview.detection.Ambiguous.suggestions;
        mapping = {
          uuid: s.Uuid?.[0] ?? null,
          klasse: s.Klasse?.[0] ?? 0,
          nachname: s.Nachname?.[0] ?? 0,
          vorname: s.Vorname?.[0] ?? 0
        };
      }
      schritt = 'preview';
    } catch (err) {
      fehler = String(err);
    }
  }

  async function anwenden() {
    if (!bytesBuffer || !mapping || ausgewaehltesSchuljahr === null) return;
    fehler = null;
    try {
      summary = await importXlsx.apply(
        ausgewaehltesSchuljahr,
        bytesBuffer,
        mapping
      );
      schritt = 'fertig';
      await refresh();
    } catch (err) {
      fehler = String(err);
    }
  }

  function zurueck() {
    schritt = 'auswahl';
    preview = null;
    mapping = null;
    bytesBuffer = null;
    summary = null;
    fehler = null;
  }
</script>

<main class="container">
  <header>
    <h1>Stammdaten-Import</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}
    <p class="error">Fehler: {fehler}</p>
  {/if}

  {#if schritt === 'auswahl'}
    <section>
      <h2>1. Schuljahr wählen</h2>
      {#if schuljahre.length > 0}
        <ul>
          {#each schuljahre as sj (sj.id)}
            <li>
              <label>
                <input
                  type="radio"
                  bind:group={ausgewaehltesSchuljahr}
                  value={sj.id}
                />
                {sj.bezeichnung}
                {sj.aktiv ? '(aktiv)' : ''}
              </label>
            </li>
          {/each}
        </ul>
      {:else}
        <p><em>Noch kein Schuljahr angelegt.</em></p>
      {/if}

      <h3>oder neu anlegen</h3>
      <input placeholder="2025/26" bind:value={neueBezeichnung} />
      <button onclick={neuesSchuljahr}>Anlegen + aktivieren</button>

      <div class="actions">
        <button onclick={weiterZuDatei} disabled={ausgewaehltesSchuljahr === null}>
          Weiter
        </button>
      </div>
    </section>
  {/if}

  {#if schritt === 'datei'}
    <section>
      <h2>2. XLSX-Datei aus ASV-BW wählen</h2>
      <p>Erwartete Spalten: ASV-UUID (optional), Klasse, Nachname, Vorname.</p>
      <input type="file" accept=".xlsx" onchange={dateiGewaehlt} />
      <button onclick={zurueck}>Abbrechen</button>
    </section>
  {/if}

  {#if schritt === 'preview' && preview && mapping}
    <section>
      <h2>3. Vorschau &amp; Spalten-Zuordnung</h2>
      <p>
        {preview.sheet.rows.length} Datenzeile(n) erkannt,
        {preview.sheet.headers.length} Spalten.
      </p>

      <table class="mapping">
        <thead>
          <tr>
            <th>Spalte in XLSX</th>
            {#each preview.sheet.headers as h, i (i)}
              <th>{i}: {h}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each ['Klasse', 'Nachname', 'Vorname', 'UUID (optional)'] as label, row (label)}
            <tr>
              <td>{label}</td>
              {#each preview.sheet.headers as _h, colIdx (colIdx)}
                <td>
                  <input
                    type="radio"
                    name={`map-${row}`}
                    checked={
                      (row === 0 && mapping.klasse === colIdx) ||
                      (row === 1 && mapping.nachname === colIdx) ||
                      (row === 2 && mapping.vorname === colIdx) ||
                      (row === 3 && mapping.uuid === colIdx)
                    }
                    onchange={() => {
                      if (!mapping) return;
                      if (row === 0) mapping = { ...mapping, klasse: colIdx };
                      else if (row === 1) mapping = { ...mapping, nachname: colIdx };
                      else if (row === 2) mapping = { ...mapping, vorname: colIdx };
                      else mapping = { ...mapping, uuid: colIdx };
                    }}
                  />
                </td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>

      <h3>Erste 5 Datenzeilen</h3>
      <table class="sample">
        <thead>
          <tr>
            {#each preview.sheet.headers as h, i (i)}
              <th>{h}</th>
            {/each}
          </tr>
        </thead>
        <tbody>
          {#each preview.sheet.rows.slice(0, 5) as row, rIdx (rIdx)}
            <tr>
              {#each row as cell, cIdx (cIdx)}
                <td>{cell}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>

      <div class="actions">
        <button onclick={zurueck}>Abbrechen</button>
        <button onclick={anwenden}>Importieren</button>
      </div>
    </section>
  {/if}

  {#if schritt === 'fertig' && summary}
    <section>
      <h2>Import abgeschlossen</h2>
      <ul>
        <li>{summary.neue_klassen} neue Klasse(n)</li>
        <li>{summary.neue_schueler} neue Schüler:innen</li>
        <li>{summary.geaenderte_schueler} aktualisiert</li>
        <li>{summary.unveraenderte_schueler} unverändert</li>
      </ul>
      <div class="actions">
        <button onclick={zurueck}>Neuer Import</button>
        <a href="/dashboard">← Dashboard</a>
      </div>
    </section>
  {/if}
</main>

<style>
  .container {
    max-width: 1000px;
    margin: 0 auto;
    padding: 2rem 1rem;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  section {
    margin-top: 2rem;
    padding: 1.5rem;
    border: 1px solid #ccc;
    border-radius: 8px;
  }
  .error {
    background: #fee;
    color: #900;
    padding: 1rem;
    border-radius: 4px;
  }
  .actions {
    margin-top: 1.5rem;
    display: flex;
    gap: 1rem;
  }
  table {
    border-collapse: collapse;
    font-size: 0.85rem;
    margin: 1rem 0;
  }
  th,
  td {
    border: 1px solid #ddd;
    padding: 4px 8px;
    text-align: left;
  }
  table.mapping td:first-child {
    font-weight: bold;
  }
  table.sample th {
    background: #f5f5f5;
  }
</style>
