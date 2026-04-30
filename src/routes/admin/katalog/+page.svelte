<!-- src/routes/admin/katalog/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import {
    stammdaten,
    katalog,
    type Schuljahr,
    type Fach,
    type Kategorie,
    type Formulierung
  } from '$lib/api';

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let tab = $state<'faecher' | 'kategorien' | 'formulierungen'>('faecher');
  let faecher = $state<Fach[]>([]);
  let kategorien = $state<Kategorie[]>([]);
  let formulierungenProKategorie = $state<Record<number, Formulierung[]>>({});
  let filterKategorieId = $state<number | null>(null);
  let neueFormulierungKategorieId = $state<number | null>(null);
  let neuName = $state('');
  let neuText = $state('');
  let fehler = $state<string | null>(null);

  onMount(async () => {
    if (session.rolle !== 'administrator') {
      goto('/dashboard');
      return;
    }
    const list = await stammdaten.list();
    aktivesSchuljahr = list.find(s => s.aktiv) ?? null;
    if (!aktivesSchuljahr) {
      fehler = 'Kein aktives Schuljahr. Bitte zuerst unter /admin/stammdaten anlegen.';
      return;
    }
    await refreshFaecher();
    await refreshKategorien();
  });

  async function refreshFaecher() {
    if (!aktivesSchuljahr) return;
    faecher = await katalog.faecher(aktivesSchuljahr.id);
  }
  async function refreshKategorien() {
    if (!aktivesSchuljahr) return;
    kategorien = await katalog.kategorien(aktivesSchuljahr.id);
    await refreshAlleFormulierungen();
  }
  async function refreshAlleFormulierungen() {
    const result: Record<number, Formulierung[]> = {};
    for (const k of kategorien) {
      result[k.id] = await katalog.formulierungen(k.id);
    }
    formulierungenProKategorie = result;
  }

  async function fachAnlegen() {
    if (!aktivesSchuljahr || !neuName.trim()) return;
    fehler = null;
    try {
      await katalog.fachAnlegen(aktivesSchuljahr.id, neuName.trim());
      neuName = '';
      await refreshFaecher();
    } catch (e) { fehler = String(e); }
  }

  let seedHinweis = $state<string | null>(null);
  let floskelnHinweis = $state<string | null>(null);
  async function seedDefaults() {
    if (!aktivesSchuljahr) return;
    fehler = null;
    seedHinweis = null;
    try {
      const sum = await katalog.seedDefaultFaecher(aktivesSchuljahr.id);
      seedHinweis = `${sum.neue_faecher} neue Fächer angelegt, ${sum.uebersprungene_faecher} übersprungen (existieren bereits).`;
      await refreshFaecher();
    } catch (e) { fehler = String(e); }
  }
  async function seedDefaultFloskeln() {
    if (!aktivesSchuljahr) return;
    fehler = null;
    floskelnHinweis = null;
    try {
      const sum = await katalog.seedDefaultFloskeln(aktivesSchuljahr.id);
      floskelnHinweis = `${sum.neue_kategorien} neue Kategorien, ${sum.neue_formulierungen} neue Formulierungen angelegt (${sum.uebersprungene_kategorien} Kat. + ${sum.uebersprungene_formulierungen} Form. übersprungen).`;
      await refreshKategorien();
    } catch (e) { fehler = String(e); }
  }
  async function kategorieAnlegen() {
    if (!aktivesSchuljahr || !neuName.trim()) return;
    fehler = null;
    try {
      await katalog.kategorieAnlegen(aktivesSchuljahr.id, neuName.trim());
      neuName = '';
      await refreshKategorien();
    } catch (e) { fehler = String(e); }
  }
  async function formulierungAnlegen() {
    if (!neueFormulierungKategorieId || !neuText.trim()) {
      fehler = 'Bitte Kategorie auswählen und Text eingeben.';
      return;
    }
    fehler = null;
    try {
      await katalog.formulierungAnlegen(neueFormulierungKategorieId, neuText.trim());
      neuText = '';
      await refreshAlleFormulierungen();
    } catch (e) { fehler = String(e); }
  }

  async function fachToggle(f: Fach) {
    await katalog.fachAktiv(f.id, !f.aktiv);
    await refreshFaecher();
  }
  async function kategorieToggle(k: Kategorie) {
    await katalog.kategorieAktiv(k.id, !k.aktiv);
    await refreshKategorien();
  }
  async function formulierungToggle(f: Formulierung) {
    await katalog.formulierungAktiv(f.id, !f.aktiv);
    await refreshAlleFormulierungen();
  }

  async function fachVerschieben(f: Fach, delta: number) {
    const idx = faecher.findIndex(x => x.id === f.id);
    const ziel = idx + delta;
    if (ziel < 0 || ziel >= faecher.length) return;
    const partner = faecher[ziel];
    await katalog.fachReihenfolge(f.id, partner.reihenfolge);
    await katalog.fachReihenfolge(partner.id, f.reihenfolge);
    await refreshFaecher();
  }
  async function kategorieVerschieben(k: Kategorie, delta: number) {
    const idx = kategorien.findIndex(x => x.id === k.id);
    const ziel = idx + delta;
    if (ziel < 0 || ziel >= kategorien.length) return;
    const partner = kategorien[ziel];
    await katalog.kategorieReihenfolge(k.id, partner.reihenfolge);
    await katalog.kategorieReihenfolge(partner.id, k.reihenfolge);
    await refreshKategorien();
  }
  async function formulierungVerschieben(f: Formulierung, delta: number) {
    const liste = formulierungenProKategorie[f.kategorie_id] ?? [];
    const idx = liste.findIndex(x => x.id === f.id);
    const ziel = idx + delta;
    if (ziel < 0 || ziel >= liste.length) return;
    const partner = liste[ziel];
    await katalog.formulierungReihenfolge(f.id, partner.reihenfolge);
    await katalog.formulierungReihenfolge(partner.id, f.reihenfolge);
    await refreshAlleFormulierungen();
  }
</script>

<main class="container">
  <header>
    <h1>Katalog-Editor</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}
    <p class="error">Fehler: {fehler}</p>
  {/if}

  {#if aktivesSchuljahr}
    <p>Aktives Schuljahr: <strong>{aktivesSchuljahr.bezeichnung}</strong></p>

    <nav class="tabs">
      <button class:aktiv={tab === 'faecher'} onclick={() => tab = 'faecher'}>Fächer</button>
      <button class:aktiv={tab === 'kategorien'} onclick={() => tab = 'kategorien'}>Kategorien</button>
      <button class:aktiv={tab === 'formulierungen'} onclick={() => tab = 'formulierungen'}>Formulierungen</button>
    </nav>

    {#if tab === 'faecher'}
      <section>
        <ul>
          {#each faecher as f (f.id)}
            <li class:inaktiv={!f.aktiv}>
              <button onclick={() => fachVerschieben(f, -1)}>↑</button>
              <button onclick={() => fachVerschieben(f, 1)}>↓</button>
              <span>{f.name}</span>
              <label>
                <input type="checkbox" checked={f.aktiv} onchange={() => fachToggle(f)} />
                aktiv
              </label>
            </li>
          {/each}
        </ul>
        <input placeholder="Neues Fach" bind:value={neuName} onkeydown={(e) => e.key === 'Enter' && fachAnlegen()} />
        <button onclick={fachAnlegen}>+ Anlegen</button>
        <p class="seed-row">
          <button onclick={seedDefaults} title="Legt die 12 Standard-Fächer aus dem ursprünglichen Jiraso an (Mathematik, Deutsch, Religion-Ethik, Erdkunde, Geschichte, Englisch, Französisch, Latein, Biologie, Sport, Musik, Bildende Kunst). Doppelte werden übersprungen.">
            Standard-Fächer anlegen
          </button>
          {#if seedHinweis}<span class="hinweis">{seedHinweis}</span>{/if}
        </p>
      </section>
    {/if}

    {#if tab === 'kategorien'}
      <section>
        <ul>
          {#each kategorien as k (k.id)}
            <li class:inaktiv={!k.aktiv}>
              <button onclick={() => kategorieVerschieben(k, -1)}>↑</button>
              <button onclick={() => kategorieVerschieben(k, 1)}>↓</button>
              <span>{k.name}</span>
              <label>
                <input type="checkbox" checked={k.aktiv} onchange={() => kategorieToggle(k)} />
                aktiv
              </label>
            </li>
          {/each}
        </ul>
        <input placeholder="Neue Kategorie" bind:value={neuName} onkeydown={(e) => e.key === 'Enter' && kategorieAnlegen()} />
        <button onclick={kategorieAnlegen}>+ Anlegen</button>
        <p class="seed-row">
          <button onclick={seedDefaultFloskeln} title="Legt die 7 Standard-Kategorien aus dem urspruenglichen Jiraso an (Lernbereitschaft, Auffassungsgabe, Beteiligung, Selbststaendigkeit/Kreativitaet, Sorgfalt, Einhalten von Regeln, Soziales Verhalten) inkl. der 27 dazugehoerigen Formulierungen. Doppelte werden uebersprungen.">
            Standard-Floskeln anlegen
          </button>
          {#if floskelnHinweis}<span class="hinweis">{floskelnHinweis}</span>{/if}
        </p>
      </section>
    {/if}

    {#if tab === 'formulierungen'}
      <section>
        <div class="filter-row">
          <label>
            Anzeigen:
            <select bind:value={filterKategorieId}>
              <option value={null}>Alle Kategorien</option>
              {#each kategorien as k (k.id)}
                <option value={k.id}>{k.name}</option>
              {/each}
            </select>
          </label>
        </div>

        {#each kategorien.filter(k => filterKategorieId === null || k.id === filterKategorieId) as k (k.id)}
          <h3 class="kategorie-header">{k.name}</h3>
          <ul>
            {#each (formulierungenProKategorie[k.id] ?? []) as f (f.id)}
              <li class:inaktiv={!f.aktiv}>
                <button onclick={() => formulierungVerschieben(f, -1)}>↑</button>
                <button onclick={() => formulierungVerschieben(f, 1)}>↓</button>
                <span>{f.text}</span>
                <label>
                  <input type="checkbox" checked={f.aktiv} onchange={() => formulierungToggle(f)} />
                  aktiv
                </label>
              </li>
            {:else}
              <li class="leer">— noch keine Formulierungen —</li>
            {/each}
          </ul>
        {/each}

        <div class="anlegen-row">
          <h3>Neue Formulierung anlegen</h3>
          <div class="anlegen-fields">
            <select bind:value={neueFormulierungKategorieId}>
              <option value={null}>— Kategorie wählen —</option>
              {#each kategorien as k (k.id)}
                <option value={k.id}>{k.name}</option>
              {/each}
            </select>
            <input
              placeholder="Text der Formulierung"
              bind:value={neuText}
              onkeydown={(e) => e.key === 'Enter' && formulierungAnlegen()}
            />
            <button onclick={formulierungAnlegen} disabled={!neueFormulierungKategorieId || !neuText.trim()}>
              + Anlegen
            </button>
          </div>
        </div>
      </section>
    {/if}
  {/if}
</main>

<style>
  .container { max-width: 900px; margin: 0 auto; padding: 2rem 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .tabs { display: flex; gap: 0.5rem; margin: 1rem 0; }
  .tabs button { padding: 0.5rem 1rem; background: #eee; border: 0; cursor: pointer; }
  .tabs button.aktiv { background: var(--sg-petrol, #004058); color: white; }
  ul { list-style: none; padding: 0; }
  li { display: flex; gap: 0.5rem; align-items: center; padding: 0.4rem; border-bottom: 1px solid #ddd; }
  li.inaktiv span { color: #999; text-decoration: line-through; }
  li button { padding: 0.2rem 0.5rem; }
  input[type="text"], input:not([type]) { padding: 0.4rem; }
  .seed-row { margin-top: 1.5rem; padding-top: 1rem; border-top: 1px dashed #ccc; display: flex; gap: 0.8rem; align-items: center; flex-wrap: wrap; }
  .hinweis { color: #060; font-size: 0.9rem; }
  .filter-row { margin-bottom: 1rem; }
  .kategorie-header { margin: 1.2rem 0 0.4rem; color: var(--sg-petrol, #004058); font-size: 1.05rem; }
  li.leer { color: #999; font-style: italic; padding: 0.4rem 0.8rem; border-bottom: 0; }
  .anlegen-row { margin-top: 2rem; padding: 1rem; background: #f4f6f8; border-radius: 6px; border: 1px solid #d0d5da; }
  .anlegen-row h3 { margin: 0 0 0.6rem; }
  .anlegen-fields { display: flex; gap: 0.5rem; align-items: center; flex-wrap: wrap; }
  .anlegen-fields select { min-width: 200px; padding: 0.4rem; }
  .anlegen-fields input { flex: 1; min-width: 250px; padding: 0.4rem; }
  .anlegen-fields button:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
