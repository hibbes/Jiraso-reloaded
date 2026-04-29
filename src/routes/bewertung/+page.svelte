<!-- src/routes/bewertung/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import { celebration } from '$lib/celebration.svelte.ts';
  import {
    stammdaten,
    katalog,
    klassenraum,
    bewertung as bewertungApi,
    bemerkung as bemerkungApi,
    type Schuljahr,
    type Klasse,
    type Fach,
    type Kategorie,
    type Formulierung,
    type SchuelerMini,
    type MatrixZelle
  } from '$lib/api';

  type Cell = {
    formulierung_id: number | null;
    geaendert_am: string | null;
    status: 'idle' | 'saving' | 'saved' | 'konflikt';
    konfliktServerFid?: number | null;
    konfliktServerTs?: string;
  };

  let aktivesSchuljahr = $state<Schuljahr | null>(null);
  let klassen = $state<Klasse[]>([]);
  let faecher = $state<Fach[]>([]);
  let kategorien = $state<Kategorie[]>([]);
  let formulierungenByKat = $state<Record<number, Formulierung[]>>({});
  let aktiveKlasse = $state<Klasse | null>(null);
  let aktivesFach = $state<Fach | null>(null);
  let schueler = $state<SchuelerMini[]>([]);
  let cells = $state<Record<string, Cell>>({}); // key: `${schueler_id}:${kategorie_id}`
  let fokusSchueler = $state<SchuelerMini | null>(null);
  let bemerkungText = $state('');
  let bemerkungStand = $state<string | null>(null);
  let bemerkungStatus = $state<'idle' | 'saving' | 'saved' | 'konflikt'>('idle');
  let bemerkungKonfliktTs = $state<string | null>(null);
  let fehler = $state<string | null>(null);

  let bemDebounce: ReturnType<typeof setTimeout> | null = null;
  let pollHandle: ReturnType<typeof setInterval> | null = null;
  let fertigToast = $state<string | null>(null);
  let fertigToastTimer: ReturnType<typeof setTimeout> | null = null;

  const klasseFertig = $derived(
    aktiveKlasse !== null && aktivesFach !== null && schueler.length > 0
      && schueler.every(s => kategorien.every(k => {
        const c = cells[`${s.id}:${k.id}`];
        return c && c.geaendert_am !== null;
      }))
  );

  function fertigKey(): string | null {
    if (!aktiveKlasse || !aktivesFach) return null;
    return `klasse-fertig:${aktiveKlasse.id}:${aktivesFach.id}`;
  }

  function wurdeGefeiert(): boolean {
    const k = fertigKey();
    return k !== null && typeof localStorage !== 'undefined' && localStorage.getItem(k) === '1';
  }

  function markGefeiert() {
    const k = fertigKey();
    if (k && typeof localStorage !== 'undefined') localStorage.setItem(k, '1');
  }

  function fertigKlick() {
    if (!aktiveKlasse || !aktivesFach) return;
    if (!klasseFertig) {
      zeigeFertigToast('Es fehlen noch Bewertungen — die Klasse ist noch nicht komplett.');
      return;
    }
    if (!wurdeGefeiert()) {
      celebration.trigger();
      markGefeiert();
      zeigeFertigToast(`✨ Klasse ${aktiveKlasse.name} – ${aktivesFach.name}: fertig!`);
    } else {
      zeigeFertigToast(`✓ Klasse ${aktiveKlasse.name} – ${aktivesFach.name} ist abgeschlossen.`);
    }
  }

  function zeigeFertigToast(msg: string) {
    if (fertigToastTimer) clearTimeout(fertigToastTimer);
    fertigToast = msg;
    fertigToastTimer = setTimeout(() => { fertigToast = null; }, 3000);
  }

  onMount(async () => {
    if (!session.rolle) { goto('/login'); return; }
    const sjs = await stammdaten.list();
    aktivesSchuljahr = sjs.find(s => s.aktiv) ?? null;
    if (!aktivesSchuljahr) { fehler = 'Kein aktives Schuljahr.'; return; }

    klassen = await klassenraum.klassen(aktivesSchuljahr.id);
    faecher = (await katalog.faecher(aktivesSchuljahr.id)).filter(f => f.aktiv);
    kategorien = (await katalog.kategorien(aktivesSchuljahr.id)).filter(k => k.aktiv);
    for (const k of kategorien) {
      formulierungenByKat[k.id] = (await katalog.formulierungen(k.id)).filter(f => f.aktiv);
    }
    pollHandle = setInterval(refreshMatrixSilent, 30_000);
  });

  $effect(() => {
    return () => { if (pollHandle) clearInterval(pollHandle); };
  });

  function key(s: number, k: number) { return `${s}:${k}`; }

  async function ladeKlasseUndFach() {
    if (!aktiveKlasse || !aktivesFach) return;
    schueler = await klassenraum.schueler(aktiveKlasse.id);
    const matrix = await bewertungApi.matrix(aktiveKlasse.id, aktivesFach.id);
    cells = {};
    for (const s of schueler) {
      for (const k of kategorien) {
        cells[key(s.id, k.id)] = { formulierung_id: null, geaendert_am: null, status: 'idle' };
      }
    }
    for (const m of matrix) {
      cells[key(m.schueler_id, m.kategorie_id)] = {
        formulierung_id: m.formulierung_id,
        geaendert_am: m.geaendert_am,
        status: 'idle',
      };
    }
  }

  async function refreshMatrixSilent() {
    if (!aktiveKlasse || !aktivesFach) return;
    const matrix = await bewertungApi.matrix(aktiveKlasse.id, aktivesFach.id);
    for (const m of matrix) {
      const k = key(m.schueler_id, m.kategorie_id);
      const c = cells[k];
      if (c && c.status !== 'saving' && c.status !== 'konflikt') {
        cells[k] = { formulierung_id: m.formulierung_id, geaendert_am: m.geaendert_am, status: 'idle' };
      }
    }
  }

  $effect(() => {
    ladeKlasseUndFach();
  });

  async function setCell(s: SchuelerMini, k: Kategorie, fid: number | null) {
    if (!aktivesFach) return;
    const ck = key(s.id, k.id);
    const before = cells[ck];
    cells[ck] = { ...before, status: 'saving', formulierung_id: fid };
    const r = await bewertungApi.set({
      schueler_id: s.id,
      fach_id: aktivesFach.id,
      kategorie_id: k.id,
      formulierung_id: fid,
      vorheriger_stand: before.geaendert_am,
    });
    if (r.status === 'Ok') {
      cells[ck] = { formulierung_id: fid, geaendert_am: r.neuer_stand || null, status: 'saved' };
      setTimeout(() => {
        if (cells[ck].status === 'saved') cells[ck] = { ...cells[ck], status: 'idle' };
      }, 1500);
    } else {
      cells[ck] = {
        formulierung_id: fid, // unsere lokale Wahl
        geaendert_am: before.geaendert_am,
        status: 'konflikt',
        konfliktServerFid: r.server_formulierung_id,
        konfliktServerTs: r.server_geaendert_am,
      };
    }
  }

  async function konfliktMeineUebernehmen(s: SchuelerMini, k: Kategorie) {
    if (!aktivesFach) return;
    const ck = key(s.id, k.id);
    const c = cells[ck];
    const r = await bewertungApi.set({
      schueler_id: s.id, fach_id: aktivesFach.id, kategorie_id: k.id,
      formulierung_id: c.formulierung_id,
      vorheriger_stand: c.konfliktServerTs ?? null,
    });
    if (r.status === 'Ok') {
      cells[ck] = { formulierung_id: c.formulierung_id, geaendert_am: r.neuer_stand || null, status: 'saved' };
    }
  }
  function konfliktIhreBehalten(s: SchuelerMini, k: Kategorie) {
    const ck = key(s.id, k.id);
    const c = cells[ck];
    cells[ck] = {
      formulierung_id: c.konfliktServerFid ?? null,
      geaendert_am: c.konfliktServerTs ?? null,
      status: 'idle',
    };
  }

  async function fokusiere(s: SchuelerMini) {
    fokusSchueler = s;
    bemerkungStatus = 'idle';
    bemerkungKonfliktTs = null;
    if (session.rolle !== 'klassenlehrer' && session.rolle !== 'administrator') return;
    const got = await bemerkungApi.get(s.id);
    if (got) { [bemerkungText, bemerkungStand] = got; }
    else { bemerkungText = ''; bemerkungStand = null; }
  }

  function bemerkungChange() {
    if (bemDebounce) clearTimeout(bemDebounce);
    bemDebounce = setTimeout(saveBemerkung, 1500);
  }
  async function saveBemerkung() {
    if (!fokusSchueler) return;
    bemerkungStatus = 'saving';
    const r = await bemerkungApi.set(fokusSchueler.id, bemerkungText, bemerkungStand);
    if (r.status === 'Ok') {
      bemerkungStand = r.neuer_stand || null;
      bemerkungStatus = 'saved';
      setTimeout(() => { if (bemerkungStatus === 'saved') bemerkungStatus = 'idle'; }, 1500);
    } else {
      bemerkungStatus = 'konflikt';
      bemerkungKonfliktTs = r.server_geaendert_am;
    }
  }
  async function bemKonfliktMeine() {
    if (!fokusSchueler) return;
    const r = await bemerkungApi.set(fokusSchueler.id, bemerkungText, bemerkungKonfliktTs);
    if (r.status === 'Ok') {
      bemerkungStand = r.neuer_stand || null;
      bemerkungStatus = 'saved';
      bemerkungKonfliktTs = null;
    }
  }
  async function bemKonfliktIhre() {
    if (!fokusSchueler) return;
    const got = await bemerkungApi.get(fokusSchueler.id);
    if (got) { [bemerkungText, bemerkungStand] = got; }
    bemerkungStatus = 'idle';
    bemerkungKonfliktTs = null;
  }

  function statusIcon(s: 'idle' | 'saving' | 'saved' | 'konflikt'): string {
    return s === 'saving' ? '⟳' : s === 'saved' ? '✓' : s === 'konflikt' ? '⚠' : '';
  }
</script>

<main class="container">
  <header>
    <h1>Bewertung</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  {#if fehler}<p class="error">{fehler}</p>{/if}

  <div class="controls">
    <label>Klasse:
      <select bind:value={aktiveKlasse}>
        <option value={null}>—</option>
        {#each klassen as k (k.id)}<option value={k}>{k.name}</option>{/each}
      </select>
    </label>
    <label>Fach:
      <select bind:value={aktivesFach}>
        <option value={null}>—</option>
        {#each faecher as f (f.id)}<option value={f}>{f.name}</option>{/each}
      </select>
    </label>
  </div>

  {#if aktiveKlasse && aktivesFach}
    <div class="fertig-row">
      <button
        class="fertig-button"
        class:bereit={klasseFertig}
        onclick={fertigKlick}
        title={klasseFertig
          ? 'Klasse als fertig markieren (Speichern passiert automatisch beim Auswählen, dieser Knopf ist nur für die Bestätigung)'
          : 'Erst alle Zellen bewerten, dann als fertig markieren'}
      >
        {#if klasseFertig && wurdeGefeiert()}
          ✓ Klasse abgeschlossen
        {:else if klasseFertig}
          ✨ Fertig — Sternenregen!
        {:else}
          Klasse fertig
        {/if}
      </button>
      <span class="fertig-hinweis">
        Bewertungen werden automatisch gespeichert, sobald du eine Auswahl triffst.
      </span>
      {#if fertigToast}
        <span class="fertig-toast">{fertigToast}</span>
      {/if}
    </div>

    <div class="grid">
      <div class="matrix">
        <table>
          <thead>
            <tr>
              <th>Schüler:in</th>
              {#each kategorien as k (k.id)}<th>{k.name}</th>{/each}
            </tr>
          </thead>
          <tbody>
            {#each schueler as s (s.id)}
              <tr class:fokus={fokusSchueler?.id === s.id} onclick={() => fokusiere(s)}>
                <td>{s.sortname}</td>
                {#each kategorien as k (k.id)}
                  {@const c = cells[key(s.id, k.id)] ?? { formulierung_id: null, geaendert_am: null, status: 'idle' }}
                  <td class:konflikt={c.status === 'konflikt'}>
                    <select
                      value={c.formulierung_id ?? ''}
                      onchange={(e) => {
                        const v = (e.target as HTMLSelectElement).value;
                        setCell(s, k, v === '' ? null : Number(v));
                      }}
                    >
                      <option value="">— keine Angabe —</option>
                      {#each (formulierungenByKat[k.id] ?? []) as f (f.id)}
                        <option value={f.id}>{f.text}</option>
                      {/each}
                    </select>
                    <span class="status">{statusIcon(c.status)}</span>
                    {#if c.status === 'konflikt'}
                      <div class="konflikt-banner">
                        Andere Sitzung hat
                        {#if c.konfliktServerFid != null}
                          „{(formulierungenByKat[k.id] ?? []).find(x => x.id === c.konfliktServerFid)?.text ?? '?'}"
                        {:else}
                          „— keine Angabe —"
                        {/if}
                        gewählt.
                        <button onclick={() => konfliktMeineUebernehmen(s, k)}>Meine</button>
                        <button onclick={() => konfliktIhreBehalten(s, k)}>Ihre</button>
                      </div>
                    {/if}
                  </td>
                {/each}
              </tr>
            {/each}
          </tbody>
        </table>
      </div>

      <aside class="panel">
        {#if fokusSchueler}
          <h2>{fokusSchueler.sortname}</h2>
          {#if session.rolle === 'klassenlehrer' || session.rolle === 'administrator'}
            <h3>Bemerkung <span class="status">{statusIcon(bemerkungStatus)}</span></h3>
            <textarea
              rows="8"
              bind:value={bemerkungText}
              oninput={bemerkungChange}
              onblur={saveBemerkung}
            ></textarea>
            <p class="counter" class:warn={bemerkungText.length > 700}>{bemerkungText.length}/700 Zeichen</p>
            {#if bemerkungStatus === 'konflikt'}
              <div class="konflikt-banner">
                Andere Sitzung hat die Bemerkung geändert.
                <button onclick={bemKonfliktMeine}>Meine</button>
                <button onclick={bemKonfliktIhre}>Ihre</button>
              </div>
            {/if}
          {:else}
            <p><em>Bemerkung nur für Klassenlehrer:in / Admin.</em></p>
          {/if}
        {:else}
          <p><em>Klick eine Schüler:in für Detail-Panel.</em></p>
        {/if}
      </aside>
    </div>
  {/if}
</main>

<style>
  .container { max-width: 1400px; margin: 0 auto; padding: 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .controls { display: flex; gap: 1rem; margin: 1rem 0; }
  .grid { display: grid; grid-template-columns: 2fr 1fr; gap: 1rem; }
  .matrix { overflow: auto; }
  table { border-collapse: collapse; font-size: 0.85rem; width: 100%; }
  th, td { border: 1px solid #ddd; padding: 0.3rem; vertical-align: top; }
  thead th { background: #f5f5f5; position: sticky; top: 0; }
  tr.fokus td { background: #fff8e1; }
  td.konflikt { background: #fee; }
  td select { max-width: 200px; }
  td .status { margin-left: 0.3rem; color: #666; }
  .konflikt-banner { background: #fee; border: 1px solid #c00; padding: 0.4rem; margin-top: 0.3rem; font-size: 0.8rem; }
  .konflikt-banner button { margin-left: 0.3rem; }
  .panel { padding: 1rem; border: 1px solid #ddd; border-radius: 4px; }
  .panel textarea { width: 100%; font-family: inherit; }
  .counter { font-size: 0.8rem; color: #666; }
  .counter.warn { color: #c00; }
  .fertig-row {
    display: flex;
    align-items: center;
    gap: 0.8rem;
    margin: 0.6rem 0 1rem;
    flex-wrap: wrap;
  }
  .fertig-button {
    padding: 0.5rem 1.1rem;
    border-radius: 6px;
    border: 1px solid #c0c0c0;
    background: #f0f0f0;
    color: #555;
    cursor: pointer;
    font-size: 0.95rem;
  }
  .fertig-button:hover { background: #e8e8e8; }
  .fertig-button.bereit {
    background: var(--sg-petrol, #004058);
    color: white;
    border-color: var(--sg-petrol, #004058);
    font-weight: 500;
  }
  .fertig-button.bereit:hover {
    background: #00567a;
    border-color: #00567a;
  }
  .fertig-hinweis { color: #666; font-size: 0.85rem; }
  .fertig-toast {
    color: #060;
    font-weight: 500;
    background: #efffe6;
    padding: 0.3rem 0.7rem;
    border-radius: 4px;
    border: 1px solid #c0e0a0;
  }
</style>
