<!-- src/routes/bewertung/+page.svelte -->
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { goto } from '$app/navigation';
  import { confirm as tauriConfirm } from '@tauri-apps/plugin-dialog';
  import { session } from '$lib/session.svelte';
  import { kuerzelStore } from '$lib/kuerzel.svelte';
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
    editor_kuerzel: string | null;
    status: 'idle' | 'saving' | 'saved' | 'konflikt';
    konfliktServerFid?: number | null;
    konfliktServerTs?: string;
    konfliktServerKuerzel?: string | null;
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

  // Kürzel-Schutz: Wer ist gerade eingetragen? Wer hat zuletzt in dieser Klasse/Fach
  // editiert? Modal beim ersten Überschreiben fremder Einträge pro (Klasse, Fach).
  let kuerzelPromptOffen = $state(false);
  let kuerzelEingabe = $state('');
  let kuerzelFehler = $state<string | null>(null);
  let letzterEditor = $state<{ kuerzel: string; ts: string } | null>(null);
  // Schon-gewarnt-Set: Pro (klasse, fach) max. einmal die Confirm-Frage ("wirklich
  // fremde Einträge überschreiben?"), damit es nicht bei jedem Klick nervt.
  let schonGewarnt = $state<Set<string>>(new Set());
  let pendingFremdSave = $state<null | {
    s: SchuelerMini; k: Kategorie; fid: number | null;
    fremdKuerzel: string; fremdTs: string;
  }>(null);

  // Panel-Modus: Schueler-Liste links + vertikaler Kategorien-Karten-Stack rechts.
  // Ziel: pro Schueler:in 7 Tastendruecke fuer komplette Bewertung.
  type ViewMode = 'panel' | 'matrix';
  function loadViewMode(): ViewMode {
    if (typeof localStorage === 'undefined') return 'panel';
    return (localStorage.getItem('bewertung-view') === 'matrix') ? 'matrix' : 'panel';
  }
  let viewMode = $state<ViewMode>(loadViewMode());
  let aktiveKategorieIndex = $state(0);
  let autoAdvance = $state(true);
  let cheatSheetOffen = $state(false);
  let bemerkungRef = $state<HTMLTextAreaElement | null>(null);
  let zuletztGespeichert = $state<{ kategorieId: number; formulierungId: number | null } | null>(null);

  function setViewMode(m: ViewMode) {
    viewMode = m;
    if (typeof localStorage !== 'undefined') localStorage.setItem('bewertung-view', m);
  }

  function toggleViewMode() { setViewMode(viewMode === 'panel' ? 'matrix' : 'panel'); }

  // Lineare Farbskala positiv -> kritisch, abhaengig von Anzahl der Pills.
  function pillFarbe(idx: number, total: number): string {
    const stufen3 = ['#34a853', '#f4b400', '#d93025'];
    const stufen4 = ['#34a853', '#9bcb45', '#f4b400', '#d93025'];
    const stufen5 = ['#34a853', '#9bcb45', '#f4b400', '#f08018', '#d93025'];
    if (total <= 3) return stufen3[Math.min(idx, 2)];
    if (total === 4) return stufen4[idx];
    return stufen5[Math.min(idx, 4)];
  }

  function progressBullet(s: SchuelerMini): '○' | '◐' | '●' {
    let n = 0;
    for (const k of kategorien) {
      const c = cells[key(s.id, k.id)];
      if (c && c.geaendert_am !== null) n++;
    }
    if (n === 0) return '○';
    if (n >= kategorien.length) return '●';
    return '◐';
  }

  function schuelerStatusKlasse(s: SchuelerMini): string {
    const b = progressBullet(s);
    return b === '●' ? 'fertig' : b === '◐' ? 'angefangen' : '';
  }

  async function naechsterSchueler() {
    if (!fokusSchueler) {
      if (schueler.length > 0) await fokusiere(schueler[0]);
      aktiveKategorieIndex = 0;
      return;
    }
    const i = schueler.findIndex(x => x.id === fokusSchueler!.id);
    if (i < schueler.length - 1) {
      await fokusiere(schueler[i + 1]);
      aktiveKategorieIndex = 0;
    }
  }
  async function vorherigerSchueler() {
    if (!fokusSchueler) return;
    const i = schueler.findIndex(x => x.id === fokusSchueler!.id);
    if (i > 0) {
      await fokusiere(schueler[i - 1]);
      aktiveKategorieIndex = 0;
    }
  }

  async function selectByIndex(formIdx: number) {
    if (!fokusSchueler) return;
    const k = kategorien[aktiveKategorieIndex];
    if (!k) return;
    const forms = formulierungenByKat[k.id] ?? [];
    if (formIdx >= forms.length) return;
    const fid = forms[formIdx].id;
    zuletztGespeichert = { kategorieId: k.id, formulierungId: fid };
    await setCell(fokusSchueler, k, fid);
    if (autoAdvance) await advanceAfterSelect();
  }

  async function selectKeineAngabe() {
    if (!fokusSchueler) return;
    const k = kategorien[aktiveKategorieIndex];
    if (!k) return;
    zuletztGespeichert = { kategorieId: k.id, formulierungId: null };
    await setCell(fokusSchueler, k, null);
    if (autoAdvance) await advanceAfterSelect();
  }

  async function advanceAfterSelect() {
    if (aktiveKategorieIndex < kategorien.length - 1) {
      aktiveKategorieIndex++;
    } else {
      // Letzte Kategorie erledigt: immer naechste:r Schueler:in, Kategorie 1.
      // Bemerkung wird NICHT auto-fokussiert; nur per Maus-Klick erreichbar.
      await naechsterSchueler();
    }
  }

  function handleKey(e: KeyboardEvent) {
    if (cheatSheetOffen) {
      if (e.key === 'Escape' || e.key === '?') {
        cheatSheetOffen = false;
        e.preventDefault();
      }
      return;
    }
    const tag = (e.target as HTMLElement).tagName;
    const inText = tag === 'TEXTAREA' || tag === 'INPUT';
    if (inText) {
      if (e.key === 'Escape') {
        (e.target as HTMLElement).blur();
        e.preventDefault();
      }
      return;
    }
    if (tag === 'SELECT') return;

    if (e.key === '?') { cheatSheetOffen = true; e.preventDefault(); return; }
    if (e.key === 'm' || e.key === 'M') { toggleViewMode(); e.preventDefault(); return; }

    if (viewMode !== 'panel') return;
    if (!aktiveKlasse || !aktivesFach) return;

    if (e.key === 'a' || e.key === 'A') {
      autoAdvance = !autoAdvance;
      zeigeFertigToast(autoAdvance ? '⏭ Auto-Advance an' : '⏸ Auto-Advance aus');
      e.preventDefault(); return;
    }
    if (e.key === 'n' || e.key === 'N' || e.key === 'ArrowDown') {
      naechsterSchueler(); e.preventDefault(); return;
    }
    if (e.key === 'p' || e.key === 'P' || e.key === 'ArrowUp') {
      vorherigerSchueler(); e.preventDefault(); return;
    }
    if (e.key === 'ArrowRight') {
      if (aktiveKategorieIndex < kategorien.length - 1) aktiveKategorieIndex++;
      e.preventDefault(); return;
    }
    if (e.key === 'ArrowLeft') {
      if (aktiveKategorieIndex > 0) aktiveKategorieIndex--;
      e.preventDefault(); return;
    }
    if (e.key === '0' || e.key === '-') {
      selectKeineAngabe(); e.preventDefault(); return;
    }
    if (/^[1-9]$/.test(e.key)) {
      selectByIndex(parseInt(e.key, 10) - 1);
      e.preventDefault();
    }
  }

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

  function fertigKlick(e: MouseEvent) {
    if (!aktiveKlasse || !aktivesFach) return;
    if (!klasseFertig) {
      zeigeFertigToast('Es fehlen noch Bewertungen — die Klasse ist noch nicht komplett.');
      return;
    }
    if (!wurdeGefeiert()) {
      const target = e.currentTarget as HTMLElement | null;
      const rect = target?.getBoundingClientRect();
      const origin = rect
        ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 }
        : null;
      celebration.trigger(origin);
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

  let wuerfelLaufend = $state(false);
  async function klasseWuerfeln() {
    if (!aktiveKlasse) return;
    // Tauri-Dialog statt window.confirm: window.confirm ist im
    // Tauri-WebView auf macOS nicht implementiert -> Klick verpufft.
    const ok = await tauriConfirm(
      `Test-Bewertungen für Klasse ${aktiveKlasse.name} würfeln?\n\nDas überschreibt bestehende Bewertungen UND Bemerkungen aller Schüler:innen dieser Klasse für ALLE Fächer.`,
      { title: 'Klasse würfeln', kind: 'warning' }
    );
    if (!ok) return;
    wuerfelLaufend = true;
    fehler = null;
    try {
      const [cells, bems] = await bewertungApi.wuerfeln(aktiveKlasse.id);
      zeigeFertigToast(`🎲 ${cells} Bewertungen + ${bems} Bemerkungen gewuerfelt.`);
      // localStorage 'gefeiert'-Marker fuer alle Faecher dieser Klasse loeschen,
      // damit der Sternenregen-Knopf wieder feiert
      if (typeof localStorage !== 'undefined') {
        for (const f of faecher) {
          localStorage.removeItem(`klasse-fertig:${aktiveKlasse.id}:${f.id}`);
        }
      }
      await ladeKlasseUndFach();
    } catch (e) {
      fehler = String(e);
    } finally {
      wuerfelLaufend = false;
    }
  }

  onMount(async () => {
    if (!session.rolle) { goto('/login'); return; }
    kuerzelStore.load();
    if (!kuerzelStore.kuerzel) {
      kuerzelPromptOffen = true;
    }
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
    window.addEventListener('keydown', handleKey);
  });

  function kuerzelBestaetigen() {
    const v = kuerzelEingabe.trim();
    if (v.length < 2 || v.length > 8) {
      kuerzelFehler = 'Bitte 2–8 Zeichen eingeben (z.B. "MZ" oder "CZE").';
      return;
    }
    kuerzelStore.set(v);
    kuerzelEingabe = '';
    kuerzelFehler = null;
    kuerzelPromptOffen = false;
  }

  function kuerzelOeffnen() {
    kuerzelEingabe = kuerzelStore.kuerzel ?? '';
    kuerzelFehler = null;
    kuerzelPromptOffen = true;
  }

  $effect(() => {
    return () => {
      if (pollHandle) clearInterval(pollHandle);
      window.removeEventListener('keydown', handleKey);
    };
  });

  // Matrix-View bekommt die volle Bildschirmbreite (16:9-Monitore),
  // Panel-View bleibt im normalen 1280px-Container.
  $effect(() => {
    if (typeof document === 'undefined') return;
    const main = document.querySelector('.app-main');
    if (!main) return;
    if (viewMode === 'matrix') main.classList.add('wide');
    else main.classList.remove('wide');
  });

  // Aktive Kategorie-Karte ins Bildschirm-Zentrum scrollen, sobald sich der
  // Index ändert (Auto-Advance, n/p, Schueler-Wechsel). So wandert der
  // Lesefokus mit, ohne dass die KuK manuell scrollen muss.
  $effect(() => {
    if (viewMode !== 'panel') return;
    if (!fokusSchueler) return;
    // Reaktivität an aktiveKategorieIndex + fokusSchueler-Wechsel knüpfen
    const idx = aktiveKategorieIndex;
    const sid = fokusSchueler.id;
    void idx; void sid;
    tick().then(() => {
      const el = document.querySelector('.kat-karte.aktiv');
      if (el && 'scrollIntoView' in el) {
        (el as HTMLElement).scrollIntoView({ block: 'center', behavior: 'smooth' });
      }
    });
  });

  function key(s: number, k: number) { return `${s}:${k}`; }

  async function ladeKlasseUndFach() {
    if (!aktiveKlasse || !aktivesFach) return;
    schueler = await klassenraum.schueler(aktiveKlasse.id);
    const matrix = await bewertungApi.matrix(aktiveKlasse.id, aktivesFach.id);
    cells = {};
    for (const s of schueler) {
      for (const k of kategorien) {
        cells[key(s.id, k.id)] = { formulierung_id: null, geaendert_am: null, editor_kuerzel: null, status: 'idle' };
      }
    }
    for (const m of matrix) {
      cells[key(m.schueler_id, m.kategorie_id)] = {
        formulierung_id: m.formulierung_id,
        geaendert_am: m.geaendert_am,
        editor_kuerzel: m.editor_kuerzel,
        status: 'idle',
      };
    }
    // Letzter Editor in dieser (Klasse, Fach)-Kombi -> Banner-Eingabe
    const le = await bewertungApi.letzterEditor(aktiveKlasse.id, aktivesFach.id);
    letzterEditor = le ? { kuerzel: le[0], ts: le[1] } : null;
  }

  async function refreshMatrixSilent() {
    if (!aktiveKlasse || !aktivesFach) return;
    const matrix = await bewertungApi.matrix(aktiveKlasse.id, aktivesFach.id);
    for (const m of matrix) {
      const k = key(m.schueler_id, m.kategorie_id);
      const c = cells[k];
      if (c && c.status !== 'saving' && c.status !== 'konflikt') {
        cells[k] = { formulierung_id: m.formulierung_id, geaendert_am: m.geaendert_am, editor_kuerzel: m.editor_kuerzel, status: 'idle' };
      }
    }
  }

  $effect(() => {
    ladeKlasseUndFach();
  });

  function fremdSchutzKey(): string | null {
    if (!aktiveKlasse || !aktivesFach) return null;
    return `${aktiveKlasse.id}:${aktivesFach.id}`;
  }

  // Wenn die Zielzelle ein FREMDES Kürzel trägt und wir in dieser (Klasse, Fach)
  // noch nicht gewarnt wurden — Modal stellen, Save aufschieben.
  async function setCell(s: SchuelerMini, k: Kategorie, fid: number | null) {
    if (!aktivesFach) return;
    if (!kuerzelStore.kuerzel) {
      kuerzelOeffnen();
      return;
    }
    const ck = key(s.id, k.id);
    const before = cells[ck];
    const fremd = before.editor_kuerzel && before.editor_kuerzel !== kuerzelStore.kuerzel;
    const fkey = fremdSchutzKey();
    if (fremd && fkey && !schonGewarnt.has(fkey)) {
      pendingFremdSave = {
        s, k, fid,
        fremdKuerzel: before.editor_kuerzel ?? '?',
        fremdTs: before.geaendert_am ?? '',
      };
      return;
    }
    await setCellRaw(s, k, fid);
  }

  async function setCellRaw(s: SchuelerMini, k: Kategorie, fid: number | null) {
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
      editor_kuerzel: kuerzelStore.kuerzel,
    });
    if (r.status === 'Ok') {
      cells[ck] = {
        formulierung_id: fid,
        geaendert_am: r.neuer_stand || null,
        editor_kuerzel: kuerzelStore.kuerzel,
        status: 'saved',
      };
      setTimeout(() => {
        if (cells[ck].status === 'saved') cells[ck] = { ...cells[ck], status: 'idle' };
      }, 1500);
      // Banner-Inhalt aktualisieren (wir sind jetzt der letzte Editor)
      if (kuerzelStore.kuerzel) {
        letzterEditor = { kuerzel: kuerzelStore.kuerzel, ts: r.neuer_stand || '' };
      }
    } else {
      cells[ck] = {
        formulierung_id: fid, // unsere lokale Wahl
        geaendert_am: before.geaendert_am,
        editor_kuerzel: before.editor_kuerzel,
        status: 'konflikt',
        konfliktServerFid: r.server_formulierung_id,
        konfliktServerTs: r.server_geaendert_am,
        konfliktServerKuerzel: r.server_editor_kuerzel,
      };
    }
  }

  async function fremdConfirmJa() {
    if (!pendingFremdSave) return;
    const { s, k, fid } = pendingFremdSave;
    const fkey = fremdSchutzKey();
    if (fkey) schonGewarnt = new Set([...schonGewarnt, fkey]);
    pendingFremdSave = null;
    await setCellRaw(s, k, fid);
  }
  function fremdConfirmAbbrechen() {
    pendingFremdSave = null;
  }

  async function konfliktMeineUebernehmen(s: SchuelerMini, k: Kategorie) {
    if (!aktivesFach) return;
    const ck = key(s.id, k.id);
    const c = cells[ck];
    const r = await bewertungApi.set({
      schueler_id: s.id, fach_id: aktivesFach.id, kategorie_id: k.id,
      formulierung_id: c.formulierung_id,
      vorheriger_stand: c.konfliktServerTs ?? null,
      editor_kuerzel: kuerzelStore.kuerzel,
    });
    if (r.status === 'Ok') {
      cells[ck] = {
        formulierung_id: c.formulierung_id,
        geaendert_am: r.neuer_stand || null,
        editor_kuerzel: kuerzelStore.kuerzel,
        status: 'saved',
      };
    }
  }
  function konfliktIhreBehalten(s: SchuelerMini, k: Kategorie) {
    const ck = key(s.id, k.id);
    const c = cells[ck];
    cells[ck] = {
      formulierung_id: c.konfliktServerFid ?? null,
      geaendert_am: c.konfliktServerTs ?? null,
      editor_kuerzel: c.konfliktServerKuerzel ?? null,
      status: 'idle',
    };
  }

  async function fokusiere(s: SchuelerMini) {
    fokusSchueler = s;
    aktiveKategorieIndex = 0;
    bemerkungStatus = 'idle';
    bemerkungKonfliktTs = null;
    if (session.rolle !== 'klassenlehrer' && session.rolle !== 'administrator') return;
    const got = await bemerkungApi.get(s.id);
    if (got) { [bemerkungText, bemerkungStand] = [got[0], got[1]]; }
    else { bemerkungText = ''; bemerkungStand = null; }
  }

  function bemerkungChange() {
    if (bemDebounce) clearTimeout(bemDebounce);
    bemDebounce = setTimeout(saveBemerkung, 1500);
  }
  async function saveBemerkung() {
    if (!fokusSchueler) return;
    bemerkungStatus = 'saving';
    const r = await bemerkungApi.set(fokusSchueler.id, bemerkungText, bemerkungStand, kuerzelStore.kuerzel);
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
    const r = await bemerkungApi.set(fokusSchueler.id, bemerkungText, bemerkungKonfliktTs, kuerzelStore.kuerzel);
    if (r.status === 'Ok') {
      bemerkungStand = r.neuer_stand || null;
      bemerkungStatus = 'saved';
      bemerkungKonfliktTs = null;
    }
  }
  async function bemKonfliktIhre() {
    if (!fokusSchueler) return;
    const got = await bemerkungApi.get(fokusSchueler.id);
    if (got) { [bemerkungText, bemerkungStand] = [got[0], got[1]]; }
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
    <button type="button" class="kuerzel-pill" onclick={kuerzelOeffnen} title="Eigenes Kürzel ändern">
      Kürzel: <strong>{kuerzelStore.kuerzel ?? '— bitte setzen —'}</strong>
    </button>
  </div>

  {#if aktiveKlasse && aktivesFach && letzterEditor && letzterEditor.kuerzel !== kuerzelStore.kuerzel}
    <div class="fremd-banner" role="alert">
      ⚠ In <strong>{aktiveKlasse.name} – {aktivesFach.name}</strong> hat zuletzt
      <strong>{letzterEditor.kuerzel}</strong> editiert ({letzterEditor.ts}).
      Du bist als <strong>{kuerzelStore.kuerzel ?? '?'}</strong> angemeldet — bist du sicher in der richtigen Klasse?
    </div>
  {/if}

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
      {#if session.rolle === 'administrator'}
        <button
          class="wuerfel-button"
          onclick={klasseWuerfeln}
          disabled={!aktiveKlasse || wuerfelLaufend}
          title="Admin-Test-Helfer: würfelt zufällige Bewertungen für alle Schüler:innen × alle Fächer × alle Kategorien"
        >
          {wuerfelLaufend ? '🎲 würfle …' : '🎲 Klasse würfeln (Admin-Test)'}
        </button>
      {/if}
      {#if fertigToast}
        <span class="fertig-toast">{fertigToast}</span>
      {/if}
      <div class="modus-row">
        <div class="modus-group" role="group" aria-label="Ansicht">
          <button
            class="modus-btn"
            class:aktiv={viewMode === 'panel'}
            onclick={() => setViewMode('panel')}
            title="Schueler-fuer-Schueler-Eingabe (Tastatur 1-5)"
          >Panel</button>
          <button
            class="modus-btn"
            class:aktiv={viewMode === 'matrix'}
            onclick={() => setViewMode('matrix')}
            title="Klassische Matrix-Uebersicht (m)"
          >Matrix</button>
        </div>
        <button
          class="hilfe-btn"
          onclick={() => cheatSheetOffen = true}
          title="Tastatur-Cheat-Sheet einblenden (?)"
        >? Tastatur</button>
      </div>
    </div>

    {#if viewMode === 'panel'}
      <div class="panel-grid">
        <aside class="schueler-liste">
          <h3>Schueler:innen</h3>
          <ul>
            {#each schueler as s (s.id)}
              <li
                class:aktiv={fokusSchueler?.id === s.id}
                class={schuelerStatusKlasse(s)}
              >
                <button onclick={() => fokusiere(s)}>
                  <span class="bullet">{progressBullet(s)}</span>
                  <span class="name">{s.sortname}</span>
                </button>
              </li>
            {/each}
          </ul>
          <p class="liste-hint">↑↓ oder n/p · m: Matrix · ?: Hilfe</p>
        </aside>

        <section class="bewertungs-panel">
          {#if fokusSchueler}
            <div class="hud">
              <strong>{fokusSchueler.sortname}</strong>
              <span>· Kategorie {aktiveKategorieIndex + 1}/{kategorien.length}</span>
              <span>· {aktivesFach?.name}</span>
              {#if !autoAdvance}<span class="auto-aus">Auto-Advance aus</span>{/if}
            </div>

            {#each kategorien as k, ki (k.id)}
              {@const c = cells[key(fokusSchueler.id, k.id)] ?? { formulierung_id: null, geaendert_am: null, status: 'idle' }}
              {@const forms = formulierungenByKat[k.id] ?? []}
              {@const aktiv = ki === aktiveKategorieIndex}
              <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions -->
              <article
                class="kat-karte"
                class:aktiv
                class:fertig={c.geaendert_am !== null}
                onclick={() => aktiveKategorieIndex = ki}
              >
                <header>
                  <span class="kat-num">{ki + 1}</span>
                  <h4>{k.name}</h4>
                  {#if c.geaendert_am !== null}<span class="kat-check">✓</span>{/if}
                  <span class="kat-status">{statusIcon(c.status)}</span>
                </header>
                <div class="pills">
                  {#each forms as f, fi (f.id)}
                    <button
                      class="pill"
                      class:gewaehlt={c.formulierung_id === f.id}
                      style:--pill-color={pillFarbe(fi, forms.length)}
                      onclick={(ev) => { ev.stopPropagation(); aktiveKategorieIndex = ki; selectByIndex(fi); }}
                      title="Taste {fi + 1}"
                    >
                      <span class="pill-num">{fi + 1}</span>
                      <span class="pill-text">{f.text}</span>
                    </button>
                  {/each}
                  <button
                    class="pill keine-angabe"
                    class:gewaehlt={c.formulierung_id === null && c.geaendert_am !== null}
                    onclick={(ev) => { ev.stopPropagation(); aktiveKategorieIndex = ki; selectKeineAngabe(); }}
                    title="Taste 0 oder -"
                  >
                    <span class="pill-num">0</span>
                    <span class="pill-text">— keine Angabe —</span>
                  </button>
                </div>
                {#if c.status === 'konflikt'}
                  <div class="konflikt-banner">
                    {#if c.konfliktServerKuerzel}
                      <strong>{c.konfliktServerKuerzel}</strong> hat
                    {:else}
                      Andere Sitzung hat
                    {/if}
                    {#if c.konfliktServerFid != null}
                      „{forms.find(x => x.id === c.konfliktServerFid)?.text ?? '?'}"
                    {:else}
                      „— keine Angabe —"
                    {/if}
                    gewaehlt.
                    <button onclick={(ev) => { ev.stopPropagation(); konfliktMeineUebernehmen(fokusSchueler!, k); }}>Meine</button>
                    <button onclick={(ev) => { ev.stopPropagation(); konfliktIhreBehalten(fokusSchueler!, k); }}>Ihre</button>
                  </div>
                {/if}
              </article>
            {/each}

            {#if session.rolle === 'klassenlehrer' || session.rolle === 'administrator'}
              <article class="bemerkungs-karte">
                <header>
                  <h4>Bemerkung <span class="kat-status">{statusIcon(bemerkungStatus)}</span></h4>
                  <span class="kat-hint">Klick ins Feld zum Bearbeiten, Esc zum Verlassen</span>
                </header>
                <textarea
                  bind:this={bemerkungRef}
                  rows="6"
                  bind:value={bemerkungText}
                  oninput={bemerkungChange}
                  onblur={saveBemerkung}
                ></textarea>
                <p class="counter" class:warn={bemerkungText.length > 700}>{bemerkungText.length}/700 Zeichen</p>
                {#if bemerkungStatus === 'konflikt'}
                  <div class="konflikt-banner">
                    Andere Sitzung hat die Bemerkung geaendert.
                    <button onclick={bemKonfliktMeine}>Meine</button>
                    <button onclick={bemKonfliktIhre}>Ihre</button>
                  </div>
                {/if}
              </article>
            {/if}
          {:else}
            <div class="leer-hint">
              <p>Klick eine Schueler:in links — oder druecke <kbd>↓</kbd> / <kbd>n</kbd>.</p>
              <p>Cheat-Sheet mit <kbd>?</kbd>.</p>
            </div>
          {/if}
        </section>
      </div>
    {:else}
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
                <td class="schueler-cell">{s.sortname}</td>
                {#each kategorien as k (k.id)}
                  {@const c = cells[key(s.id, k.id)] ?? { formulierung_id: null, geaendert_am: null, status: 'idle' }}
                  {@const forms = formulierungenByKat[k.id] ?? []}
                  {@const fIdx = c.formulierung_id != null ? forms.findIndex(f => f.id === c.formulierung_id) : -1}
                  {@const farbe = fIdx >= 0 ? pillFarbe(fIdx, forms.length) : null}
                  <td
                    class:konflikt={c.status === 'konflikt'}
                    class:gewaehlt={fIdx >= 0}
                    class:keine-angabe={c.formulierung_id === null && c.geaendert_am !== null}
                    style:--cell-color={farbe}
                  >
                    <select
                      value={c.formulierung_id ?? ''}
                      onchange={(e) => {
                        const v = (e.target as HTMLSelectElement).value;
                        setCell(s, k, v === '' ? null : Number(v));
                      }}
                    >
                      <option value="">— keine Angabe —</option>
                      {#each forms as f (f.id)}
                        <option value={f.id}>{f.text}</option>
                      {/each}
                    </select>
                    <span class="status">{statusIcon(c.status)}</span>
                    {#if c.status === 'konflikt'}
                      <div class="konflikt-banner">
                        {#if c.konfliktServerKuerzel}
                          <strong>{c.konfliktServerKuerzel}</strong> hat
                        {:else}
                          Andere Sitzung hat
                        {/if}
                        {#if c.konfliktServerFid != null}
                          „{forms.find(x => x.id === c.konfliktServerFid)?.text ?? '?'}"
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
  {/if}

  {#if kuerzelPromptOffen}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal-overlay" onclick={() => { if (kuerzelStore.kuerzel) kuerzelPromptOffen = false; }}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="modal-content" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
        <h2>Wer arbeitet hier?</h2>
        <p>
          Bitte gib dein Kürzel ein (2–8 Zeichen, z.&nbsp;B. <code>MZ</code> oder <code>CZE</code>).
          Es wird an jede Bewertung gehängt — andere Lehrkräfte sehen dann, dass du der letzte Editor warst,
          und werden gewarnt, bevor sie deine Eingaben überschreiben.
        </p>
        <form onsubmit={(e) => { e.preventDefault(); kuerzelBestaetigen(); }}>
          <label>
            Mein Kürzel
            <input
              type="text"
              bind:value={kuerzelEingabe}
              maxlength="8"
              autofocus
              autocomplete="off"
              required
            />
          </label>
          {#if kuerzelFehler}
            <p class="modal-error">{kuerzelFehler}</p>
          {/if}
          <div class="modal-actions">
            {#if kuerzelStore.kuerzel}
              <button type="button" onclick={() => { kuerzelPromptOffen = false; kuerzelFehler = null; }}>Abbrechen</button>
            {/if}
            <button type="submit" class="primary">Übernehmen</button>
          </div>
        </form>
      </div>
    </div>
  {/if}

  {#if pendingFremdSave}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="modal-overlay" onclick={fremdConfirmAbbrechen}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="modal-content fremd-modal" onclick={(e) => e.stopPropagation()} role="alertdialog" tabindex="-1">
        <h2>Fremde Bewertung überschreiben?</h2>
        <p>
          In dieser Zelle hat <strong>{pendingFremdSave.fremdKuerzel}</strong> am
          <strong>{pendingFremdSave.fremdTs}</strong> bereits etwas eingetragen. Du bist als
          <strong>{kuerzelStore.kuerzel}</strong> angemeldet.
        </p>
        <p>
          Bist du wirklich in der <strong>richtigen Klasse</strong>?
          Falls ja, gilt die Frage für die ganze Sitzung als beantwortet — du wirst beim
          nächsten fremden Eintrag nicht erneut gefragt.
        </p>
        <div class="modal-actions">
          <button type="button" onclick={fremdConfirmAbbrechen}>Abbrechen</button>
          <button type="button" class="primary danger" onclick={fremdConfirmJa}>
            Ja, überschreiben (für ganze Sitzung bestätigen)
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if cheatSheetOffen}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions a11y_no_static_element_interactions -->
    <div class="cheat-overlay" onclick={() => cheatSheetOffen = false}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_noninteractive_element_interactions a11y_no_static_element_interactions -->
      <div class="cheat-modal" onclick={(e) => e.stopPropagation()} role="dialog" tabindex="-1">
        <header>
          <h2>Tastatur-Cheat-Sheet</h2>
          <button class="cheat-close" onclick={() => cheatSheetOffen = false} title="Schliessen (Esc / ?)">✕</button>
        </header>
        <div class="cheat-grid">
          <section>
            <h3>Auswahl</h3>
            <dl>
              <dt><kbd>1</kbd> – <kbd>5</kbd></dt><dd>Formulierung in aktiver Kategorie</dd>
              <dt><kbd>0</kbd> oder <kbd>-</kbd></dt><dd>keine Angabe</dd>
            </dl>
          </section>
          <section>
            <h3>Navigation</h3>
            <dl>
              <dt><kbd>↑</kbd> / <kbd>↓</kbd></dt><dd>Schueler:in vor/zurueck</dd>
              <dt><kbd>n</kbd> / <kbd>p</kbd></dt><dd>dito (alternative Tasten)</dd>
              <dt><kbd>←</kbd> / <kbd>→</kbd></dt><dd>Kategorie ohne Auswahl wechseln</dd>
            </dl>
          </section>
          <section>
            <h3>Modus</h3>
            <dl>
              <dt><kbd>m</kbd></dt><dd>Panel ⇄ Matrix</dd>
              <dt><kbd>a</kbd></dt><dd>Auto-Advance an/aus</dd>
              <dt><kbd>?</kbd></dt><dd>diese Hilfe</dd>
              <dt><kbd>Esc</kbd></dt><dd>Hilfe schliessen / Bemerkung verlassen</dd>
            </dl>
          </section>
          <section>
            <h3>Bemerkung (Klassenlehrer:in)</h3>
            <dl>
              <dt>Maus-Klick</dt><dd>Cursor ins Bemerkungsfeld</dd>
              <dt><kbd>Esc</kbd></dt><dd>Bemerkung verlassen</dd>
            </dl>
          </section>
        </div>
        <footer>
          <p>Auto-Advance: nach jeder Auswahl springt der Fokus zur naechsten Kategorie. Nach der letzten Kategorie geht es direkt zur naechsten Schueler:in (Kategorie 1). Bemerkung nur per Maus-Klick.</p>
        </footer>
      </div>
    </div>
  {/if}
</main>

<style>
  .container { max-width: 1400px; margin: 0 auto; padding: 1rem; }
  header { display: flex; justify-content: space-between; align-items: center; }
  .error { background: #fee; color: #900; padding: 1rem; border-radius: 4px; }
  .controls { display: flex; gap: 1rem; align-items: center; margin: 1rem 0; flex-wrap: wrap; }
  .kuerzel-pill {
    margin-left: auto;
    padding: 0.4rem 0.9rem;
    border: 1px solid var(--sg-border, #cfd6dd);
    border-radius: 999px;
    background: var(--sg-surface, #fff);
    color: var(--sg-text, #1a1d22);
    cursor: pointer;
    font-size: 0.88rem;
  }
  .kuerzel-pill:hover { background: var(--sg-bg-card, #f3f5f7); }
  .kuerzel-pill strong { font-family: ui-monospace, monospace; }

  .fremd-banner {
    margin: 0 0 1rem;
    padding: 0.7rem 1rem;
    border: 1px solid #d8a000;
    background: #fff8e0;
    color: #5a4400;
    border-radius: 6px;
    font-size: 0.92rem;
  }

  .modal-overlay {
    position: fixed; inset: 0; background: rgba(20,20,30,0.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 1100; padding: 1rem;
  }
  .modal-content {
    background: var(--sg-surface, #fff); color: var(--sg-text, #1a1d22);
    border-radius: 10px; max-width: 540px; width: 100%;
    box-shadow: 0 12px 40px rgba(0,0,0,0.3);
    padding: 1.4rem 1.5rem;
  }
  .modal-content h2 { margin: 0 0 0.6rem; font-size: 1.15rem; }
  .modal-content p { margin: 0 0 0.7rem; line-height: 1.4; }
  .modal-content code {
    background: var(--sg-bg-card, #f3f5f7); padding: 0.05rem 0.3rem;
    border-radius: 3px; font-family: ui-monospace, monospace; font-size: 0.92em;
  }
  .modal-content label { display: flex; flex-direction: column; gap: 0.3rem; margin-top: 0.4rem; }
  .modal-content input[type="text"] {
    padding: 0.5rem 0.6rem; border: 1px solid var(--sg-border, #cfd6dd);
    border-radius: 4px; font-size: 1.05rem; font-family: ui-monospace, monospace;
    text-transform: uppercase;
  }
  .modal-error { color: #c00; margin: 0.5rem 0 0; font-size: 0.88rem; }
  .modal-actions {
    display: flex; gap: 0.6rem; justify-content: flex-end; margin-top: 1rem;
    flex-wrap: wrap;
  }
  .modal-actions .primary {
    background: var(--sg-petrol, #2a7ea1); color: #fff; border-color: var(--sg-petrol, #2a7ea1);
  }
  .modal-actions .primary:hover { background: var(--sg-petrol-hover, #1f6480); }
  .modal-actions .primary.danger { background: #c0392b; border-color: #c0392b; }
  .modal-actions .primary.danger:hover { background: #a02818; }
  .fremd-modal { border-top: 4px solid #d8a000; }
  .grid { display: grid; grid-template-columns: minmax(0, 1fr) 22rem; gap: 1rem; }
  .matrix { overflow: auto; }
  table { border-collapse: collapse; font-size: 0.92rem; width: 100%; table-layout: fixed; }
  th, td { border: 1px solid var(--sg-border); padding: 0.4rem; vertical-align: middle; }
  thead th { background: var(--sg-bg-card); position: sticky; top: 0; color: var(--sg-petrol); font-size: 0.85rem; }
  th.schueler-cell, td.schueler-cell { width: 12rem; font-weight: 500; }
  tr.fokus td { background: var(--sg-gold-soft, #fff8e1); }
  td.konflikt { background: #ffe8e8 !important; }
  td.gewaehlt {
    background: color-mix(in srgb, var(--cell-color) 14%, transparent);
    border-left: 4px solid var(--cell-color);
  }
  td.keine-angabe { background: #f4f4f6; font-style: italic; }
  td select {
    width: 100%;
    max-width: none;
    font-size: 0.9rem;
    padding: 0.32rem 0.5rem;
    background: white;
  }
  td.gewaehlt select {
    background: color-mix(in srgb, var(--cell-color) 8%, white);
    border-color: var(--cell-color);
    font-weight: 500;
  }
  td .status { margin-left: 0.3rem; color: var(--sg-meta); font-size: 0.85em; }
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
  .wuerfel-button {
    margin-left: auto;
    padding: 0.4rem 0.9rem;
    border-radius: 6px;
    border: 1px dashed #b0a060;
    background: #fff8de;
    color: #6b5a10;
    cursor: pointer;
    font-size: 0.88rem;
  }
  .wuerfel-button:hover { background: #fff2b8; }
  .wuerfel-button:disabled { opacity: 0.5; cursor: not-allowed; }

  /* Modus-Toggle */
  .modus-row { display: flex; align-items: center; gap: 0.6rem; margin-left: auto; }
  .modus-group { display: inline-flex; border: 1px solid #ccc; border-radius: 6px; overflow: hidden; }
  .modus-btn {
    padding: 0.35rem 0.9rem;
    border: 0;
    background: #fafafa;
    cursor: pointer;
    font-size: 0.88rem;
    color: #555;
  }
  .modus-btn.aktiv { background: var(--sg-petrol, #004058); color: white; }
  .modus-btn:not(.aktiv):hover { background: #eee; }
  .hilfe-btn {
    padding: 0.35rem 0.9rem;
    border: 1px solid #ccc;
    border-radius: 6px;
    background: #fafafa;
    cursor: pointer;
    font-size: 0.88rem;
    color: #555;
  }
  .hilfe-btn:hover { background: #eee; }

  /* Panel-Modus Layout */
  .panel-grid { display: grid; grid-template-columns: 16rem 1fr; gap: 1.2rem; align-items: start; }

  .schueler-liste {
    border: 1px solid #ddd;
    border-radius: 6px;
    background: #fafafa;
    padding: 0.5rem;
    position: sticky;
    top: 0.5rem;
    max-height: calc(100vh - 4rem);
    overflow-y: auto;
  }
  .schueler-liste h3 { margin: 0.2rem 0 0.5rem; font-size: 0.88rem; color: #555; text-transform: uppercase; letter-spacing: 0.05em; }
  .schueler-liste ul { list-style: none; padding: 0; margin: 0; }
  .schueler-liste li { margin: 0; }
  .schueler-liste li button {
    width: 100%;
    text-align: left;
    border: 0;
    background: transparent;
    color: var(--sg-text);
    padding: 0.35rem 0.5rem;
    font: inherit;
    cursor: pointer;
    border-radius: 4px;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    box-shadow: none;
  }
  .schueler-liste li button:hover { background: var(--sg-bg-card); color: var(--sg-text); box-shadow: none; }
  .schueler-liste li.aktiv button {
    background: var(--sg-gold-soft, #fff3c1);
    color: var(--sg-text);
    font-weight: 600;
    box-shadow: inset 3px 0 0 var(--sg-gold, #c9a747);
  }
  .schueler-liste li.fertig .bullet { color: #060; }
  .schueler-liste li.angefangen .bullet { color: #b07020; }
  .schueler-liste .bullet { font-size: 1.1rem; line-height: 1; width: 1.2rem; text-align: center; color: #999; }
  .schueler-liste .name { flex: 1; }
  .liste-hint { font-size: 0.72rem; color: #888; margin: 0.6rem 0.2rem 0; }

  .bewertungs-panel { display: flex; flex-direction: column; gap: 0.7rem; }
  .hud {
    display: flex; gap: 0.5rem; align-items: baseline;
    background: #f7f7fa; border: 1px solid #e0e0e8; padding: 0.4rem 0.7rem;
    border-radius: 6px; font-size: 0.92rem;
  }
  .hud .auto-aus { margin-left: auto; color: #b07020; font-size: 0.8rem; font-style: italic; }

  .leer-hint {
    border: 1px dashed #ccc; border-radius: 8px; padding: 2rem; text-align: center; color: #777;
  }
  .leer-hint kbd {
    display: inline-block; padding: 0.1rem 0.4rem; border: 1px solid #aaa;
    border-bottom-width: 2px; border-radius: 3px; background: #f5f5f5;
    font-family: ui-monospace, monospace; font-size: 0.85em;
  }

  .kat-karte {
    border: 1px solid #ddd;
    border-radius: 8px;
    padding: 0.6rem 0.8rem;
    background: white;
    transition: border-color 0.12s, box-shadow 0.12s, background 0.12s;
    cursor: pointer;
  }
  .kat-karte:not(.aktiv) { opacity: 0.78; }
  .kat-karte.aktiv {
    border-color: var(--sg-petrol, #004058);
    box-shadow: 0 0 0 2px var(--sg-petrol, #004058), 0 4px 12px rgba(0,64,88,0.12);
    opacity: 1;
    position: relative;
  }
  .kat-karte.aktiv::before {
    content: '▶'; position: absolute; left: -1.5rem; top: 0.7rem;
    color: var(--sg-petrol, #004058); font-size: 1.1rem;
  }
  .kat-karte.fertig { background: #fafffa; }
  .kat-karte > header {
    display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.45rem;
  }
  .kat-karte h4 { margin: 0; font-size: 0.95rem; flex: 1; }
  .kat-num {
    width: 1.5rem; height: 1.5rem; border-radius: 50%;
    background: #eef0f4; color: #555;
    display: inline-flex; align-items: center; justify-content: center;
    font-size: 0.78rem; font-weight: 600;
  }
  .kat-karte.aktiv .kat-num { background: var(--sg-petrol, #004058); color: white; }
  .kat-check { color: #060; font-weight: 600; }
  .kat-status { color: #666; font-size: 0.85em; }
  .kat-hint { color: #888; font-size: 0.78rem; margin-left: auto; }

  .pills { display: flex; flex-direction: column; gap: 0.25rem; }
  .pill {
    --pill-color: #888;
    display: flex; align-items: stretch; gap: 0.5rem;
    border: 2px solid transparent;
    border-radius: 6px;
    background: #f6f6f8;
    padding: 0.32rem 0.5rem 0.32rem 0;
    text-align: left;
    font: inherit; font-size: 0.88rem; color: #333;
    cursor: pointer;
    transition: background 0.1s, border-color 0.1s;
  }
  .pill:hover { background: #ececf2; }
  .pill .pill-num {
    width: 1.7rem; flex: 0 0 auto;
    display: inline-flex; align-items: center; justify-content: center;
    font-weight: 600; color: var(--pill-color);
    border-right: 3px solid var(--pill-color);
    font-family: ui-monospace, monospace;
  }
  .pill .pill-text { flex: 1; line-height: 1.25; padding: 0.05rem 0; }
  .pill.gewaehlt {
    background: color-mix(in srgb, var(--pill-color) 18%, white);
    border-color: var(--pill-color);
    font-weight: 500;
  }
  .pill.gewaehlt .pill-num { color: white; background: var(--pill-color); border-right-color: var(--pill-color); }
  .pill.keine-angabe { --pill-color: #888; font-style: italic; color: #555; }

  .bemerkungs-karte {
    border: 1px solid #ddd; border-radius: 8px; padding: 0.6rem 0.8rem; background: white;
    margin-top: 0.5rem;
  }
  .bemerkungs-karte > header { display: flex; align-items: baseline; gap: 0.5rem; margin-bottom: 0.4rem; }
  .bemerkungs-karte h4 { margin: 0; font-size: 0.95rem; flex: 1; }
  .bemerkungs-karte textarea {
    width: 100%; box-sizing: border-box; font-family: inherit; font-size: 0.92rem;
    padding: 0.4rem 0.5rem; border: 1px solid #ccc; border-radius: 4px;
  }
  .bemerkungs-karte textarea:focus { outline: 2px solid var(--sg-petrol, #004058); outline-offset: 1px; }

  /* Cheat-Sheet Overlay */
  .cheat-overlay {
    position: fixed; inset: 0; background: rgba(20,20,30,0.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 1000; padding: 1rem;
  }
  .cheat-modal {
    background: white; border-radius: 10px; max-width: 720px; width: 100%;
    box-shadow: 0 12px 40px rgba(0,0,0,0.3); padding: 1.2rem 1.4rem;
    max-height: 90vh; overflow-y: auto;
  }
  .cheat-modal > header { display: flex; align-items: center; gap: 1rem; margin-bottom: 0.8rem; }
  .cheat-modal h2 { margin: 0; font-size: 1.15rem; flex: 1; }
  .cheat-close {
    border: 0; background: transparent; font-size: 1.2rem; cursor: pointer; color: #888;
    padding: 0.2rem 0.4rem;
  }
  .cheat-close:hover { color: #000; }
  .cheat-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1.2rem 1.6rem; }
  .cheat-grid section h3 { margin: 0 0 0.4rem; font-size: 0.85rem; color: #555; text-transform: uppercase; letter-spacing: 0.05em; }
  .cheat-grid dl { margin: 0; display: grid; grid-template-columns: auto 1fr; gap: 0.25rem 0.7rem; align-items: baseline; }
  .cheat-grid dt { white-space: nowrap; }
  .cheat-grid dd { margin: 0; color: #444; }
  kbd {
    display: inline-block; padding: 0.05rem 0.4rem; border: 1px solid #aaa;
    border-bottom-width: 2px; border-radius: 3px; background: #f5f5f5;
    font-family: ui-monospace, monospace; font-size: 0.85em;
    line-height: 1; vertical-align: 0.05em;
  }
  .cheat-modal footer { margin-top: 1rem; padding-top: 0.7rem; border-top: 1px solid #eee; color: #666; font-size: 0.85rem; }
  .cheat-modal footer p { margin: 0; }
</style>
