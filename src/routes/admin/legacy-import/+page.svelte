<!-- src/routes/admin/legacy-import/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    legacyImport,
    type Schuljahr,
    type LegacyImportPreview,
    type LegacyImportSummary
  } from '$lib/api';

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let faecherBytes = $state<number[] | null>(null);
  let floskelnBytes = $state<number[] | null>(null);
  let formatBytes = $state<number[] | null>(null);
  let preview = $state<LegacyImportPreview | null>(null);
  let summary = $state<LegacyImportSummary | null>(null);
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (session.rolle !== 'administrator') { goto('/dashboard'); return; }
    const list = await stammdaten.list();
    aktivesSchuljahr = list.find(s => s.aktiv) ?? null;
  });

  async function fileBytes(e: Event): Promise<number[] | null> {
    const f = (e.target as HTMLInputElement).files?.[0];
    if (!f) return null;
    return Array.from(new Uint8Array(await f.arrayBuffer()));
  }

  async function vorschauZeigen() {
    if (!faecherBytes || !floskelnBytes || !formatBytes) {
      fehler = 'Bitte alle drei Dateien wählen.';
      return;
    }
    fehler = null;
    try {
      preview = await legacyImport.preview(faecherBytes, floskelnBytes, formatBytes);
    } catch (e) { fehler = String(e); }
  }

  async function importieren() {
    if (!aktivesSchuljahr || !preview) return;
    fehler = null;
    try {
      summary = await legacyImport.apply(aktivesSchuljahr.id, preview);
    } catch (e) { fehler = String(e); }
  }
</script>

<main class="container">
  <header>
    <h1>Legacy-Import (Fächer + Floskeln + format.xls)</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}<p class="error">Fehler: {fehler}</p>{/if}

  {#if !summary}
    <section>
      <p>Aktives Schuljahr: <strong>{aktivesSchuljahr?.bezeichnung ?? '—'}</strong></p>
      <p>Wähle die drei Dateien aus dem alten Jiraso-ProgrammPaket:</p>
      <ol>
        <li>
          <label>Fächer.txt: <input type="file" accept=".txt" onchange={async (e) => faecherBytes = await fileBytes(e)} /></label>
          {#if faecherBytes} ✓ {faecherBytes.length} Bytes {/if}
        </li>
        <li>
          <label>Floskeln.txt: <input type="file" accept=".txt" onchange={async (e) => floskelnBytes = await fileBytes(e)} /></label>
          {#if floskelnBytes} ✓ {floskelnBytes.length} Bytes {/if}
        </li>
        <li>
          <label>format.xls: <input type="file" accept=".xls,.xlsx" onchange={async (e) => formatBytes = await fileBytes(e)} /></label>
          {#if formatBytes} ✓ {formatBytes.length} Bytes {/if}
        </li>
      </ol>
      <button onclick={vorschauZeigen}>Vorschau anzeigen</button>

      {#if preview}
        <h2>Vorschau</h2>
        <h3>Fächer ({preview.faecher.length})</h3>
        <ul>{#each preview.faecher as f (f)}<li>{f}</li>{/each}</ul>
        <h3>Kategorien ({preview.kategorien.length})</h3>
        {#each preview.kategorien as k (k.name)}
          <h4>{k.name}</h4>
          <ul>{#each k.formulierungen as f (f)}<li>{f}</li>{/each}</ul>
        {/each}

        <button onclick={importieren}>Importieren</button>
      {/if}
    </section>
  {:else}
    <section>
      <h2>Import abgeschlossen</h2>
      <ul>
        <li>{summary.neue_faecher} neue Fächer ({summary.uebersprungene_faecher} übersprungen)</li>
        <li>{summary.neue_kategorien} neue Kategorien ({summary.uebersprungene_kategorien} übersprungen)</li>
        <li>{summary.neue_formulierungen} neue Formulierungen ({summary.uebersprungene_formulierungen} übersprungen)</li>
      </ul>
      <a href="/admin/katalog">→ Katalog-Editor</a>
    </section>
  {/if}
</main>

<style>
  .container { max-width: 800px; margin: 0 auto; padding: 2rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  ol li { margin: 0.5rem 0; }
  ul { margin-left: 1rem; }
</style>
