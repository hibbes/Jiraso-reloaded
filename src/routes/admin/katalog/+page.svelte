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
  let aktiveKategorie = $state<Kategorie | null>(null);
  let formulierungen = $state<Formulierung[]>([]);
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
  }
  async function refreshFormulierungen() {
    if (!aktiveKategorie) { formulierungen = []; return; }
    formulierungen = await katalog.formulierungen(aktiveKategorie.id);
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
    if (!aktiveKategorie || !neuText.trim()) return;
    fehler = null;
    try {
      await katalog.formulierungAnlegen(aktiveKategorie.id, neuText.trim());
      neuText = '';
      await refreshFormulierungen();
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
    await refreshFormulierungen();
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
    const idx = formulierungen.findIndex(x => x.id === f.id);
    const ziel = idx + delta;
    if (ziel < 0 || ziel >= formulierungen.length) return;
    const partner = formulierungen[ziel];
    await katalog.formulierungReihenfolge(f.id, partner.reihenfolge);
    await katalog.formulierungReihenfolge(partner.id, f.reihenfolge);
    await refreshFormulierungen();
  }

  $effect(() => {
    refreshFormulierungen();
  });
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
      </section>
    {/if}

    {#if tab === 'formulierungen'}
      <section>
        <label>
          Kategorie:
          <select onchange={(e) => aktiveKategorie = kategorien.find(k => k.id === Number((e.target as HTMLSelectElement).value)) ?? null}>
            <option value="">— wählen —</option>
            {#each kategorien as k (k.id)}
              <option value={k.id}>{k.name}</option>
            {/each}
          </select>
        </label>

        {#if aktiveKategorie}
          <ul>
            {#each formulierungen as f (f.id)}
              <li class:inaktiv={!f.aktiv}>
                <button onclick={() => formulierungVerschieben(f, -1)}>↑</button>
                <button onclick={() => formulierungVerschieben(f, 1)}>↓</button>
                <span>{f.text}</span>
                <label>
                  <input type="checkbox" checked={f.aktiv} onchange={() => formulierungToggle(f)} />
                  aktiv
                </label>
              </li>
            {/each}
          </ul>
          <input placeholder="Neue Formulierung" bind:value={neuText} onkeydown={(e) => e.key === 'Enter' && formulierungAnlegen()} />
          <button onclick={formulierungAnlegen}>+ Anlegen</button>
        {/if}
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
</style>
