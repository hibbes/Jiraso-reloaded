<!-- src/routes/+layout.svelte — Schiller-CI Corporate Design -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { session } from '$lib/session.svelte';
  import { schulname, aktuellesSchuljahr, currentRole, logout } from '$lib/api';
  import { kuerzelStore } from '$lib/kuerzel.svelte';
  import { goto } from '$app/navigation';
  import '../app.css';
  import BugButton from '$lib/BugButton.svelte';
  import Celebration from '$lib/Celebration.svelte';

  // __APP_VERSION__ wird von vite via define() injectiert (siehe vite.config.js).
  // Wir greifen ueber globalThis zu, damit svelte-check nicht ueber das
  // declare-Statement im Modul-Kopf stolpert (das gehoert in context=module).
  const appVersion = (globalThis as any).__APP_VERSION__ ?? '0.0.0';

  // Auto-Logout nach Inaktivitaet. Schuetzt den Lock-Slot auf Z:\, falls
  // jemand die App offen laesst (Pause, Klassen-Wechsel) -- nach 10min
  // ohne Maus/Tastatur-Event wird abgemeldet, der Slot ist wieder frei.
  const IDLE_TIMEOUT_MS = 10 * 60 * 1000;
  const IDLE_WARN_MS = 30 * 1000;
  let idleHandle: ReturnType<typeof setTimeout> | null = null;
  let warnHandle: ReturnType<typeof setTimeout> | null = null;
  let warnSichtbar = $state(false);

  function clearIdleTimers() {
    if (idleHandle) { clearTimeout(idleHandle); idleHandle = null; }
    if (warnHandle) { clearTimeout(warnHandle); warnHandle = null; }
    warnSichtbar = false;
  }
  function armIdleTimers() {
    clearIdleTimers();
    if (!session.rolle) return;
    warnHandle = setTimeout(() => { warnSichtbar = true; }, IDLE_TIMEOUT_MS - IDLE_WARN_MS);
    idleHandle = setTimeout(() => { handleLogout('idle'); }, IDLE_TIMEOUT_MS);
  }
  function onActivity() {
    if (!session.rolle) return;
    armIdleTimers();
  }

  let theme = $state<'light' | 'dark'>('light');

  function applyTheme(t: 'light' | 'dark') {
    theme = t;
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = t;
    }
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem('jiraso-theme', t);
    }
  }
  function toggleTheme() { applyTheme(theme === 'dark' ? 'light' : 'dark'); }

  onMount(async () => {
    const saved = localStorage.getItem('jiraso-theme');
    if (saved === 'dark' || saved === 'light') {
      applyTheme(saved);
    } else if (typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches) {
      applyTheme('dark');
    } else {
      applyTheme('light');
    }
    session.schule = await schulname();
    session.schuljahr = await aktuellesSchuljahr();
    session.rolle = await currentRole();
    if (session.rolle) armIdleTimers();
    for (const ev of ['mousemove', 'mousedown', 'keydown', 'touchstart', 'wheel']) {
      window.addEventListener(ev, onActivity, { passive: true });
    }
  });

  // Cleanup separat ueber $effect.root, damit onMount keinen Promise<()=>void> zurueckgibt.
  $effect(() => {
    return () => {
      if (typeof window === 'undefined') return;
      for (const ev of ['mousemove', 'mousedown', 'keydown', 'touchstart', 'wheel']) {
        window.removeEventListener(ev, onActivity);
      }
      clearIdleTimers();
    };
  });

  $effect(() => {
    // Wenn sich session.rolle aendert (z.B. nach Login auf einer
    // anderen Seite), Idle-Timer entsprechend an/aus.
    if (session.rolle) armIdleTimers(); else clearIdleTimers();
  });

  async function handleLogout(grund: 'klick' | 'idle' = 'klick') {
    clearIdleTimers();
    await logout();
    session.rolle = null;
    // Kuerzel ist an die Person gebunden, nicht an den PC -- bei Abmeldung
    // wegwerfen, damit die naechste Lehrkraft am gleichen Rechner explizit
    // ihr eigenes Kuerzel setzen muss (und nicht aus Versehen unter fremdem
    // Kuerzel speichert).
    kuerzelStore.clear();
    if (grund === 'idle') {
      // Nur den Login-Pfad mit Hinweis-Query oeffnen, damit User weiss warum.
      goto('/login?grund=idle');
    } else {
      goto('/login');
    }
  }

  let { children } = $props();
</script>

<header class="app-header">
  <div class="header-left">
    <div class="brand-mark" aria-hidden="true"></div>
    <div class="brand-text">
      <div class="brand-name">{session.schule || 'Schiller-Gymnasium'}</div>
      <div class="brand-tagline">Verbalbeurteilungen 5/6 · SJ {session.schuljahr || '…'}</div>
    </div>
  </div>
  <div class="header-right">
    <a href="/hilfe" class="btn-hilfe" title="Anleitung für Lehrkräfte">? Hilfe</a>
    <button
      class="btn-theme"
      onclick={toggleTheme}
      title={theme === 'dark' ? 'Hell-Modus' : 'Dark-Modus'}
      aria-label={theme === 'dark' ? 'Hell-Modus' : 'Dark-Modus'}
    >
      {theme === 'dark' ? '☀' : '☾'}
    </button>
    {#if session.rolle}
      <span class="badge role-{session.rolle}">{session.rolle}</span>
      <button class="btn-logout" onclick={() => handleLogout('klick')}>Abmelden</button>
    {:else}
      <span class="badge muted">nicht angemeldet</span>
    {/if}
  </div>
</header>

{#if warnSichtbar}
  <div class="idle-warn" role="alert" aria-live="polite">
    Du wirst in &lt;30s automatisch abgemeldet (10min ohne Aktivitaet).
    Maus bewegen oder Taste druecken, um angemeldet zu bleiben.
  </div>
{/if}

<main class="app-main">
  {@render children()}
</main>

<footer class="app-footer">
  <span>Jiraso-reloaded</span>
  <span class="sep">·</span>
  <span class="mono">v{appVersion}</span>
  <span class="sep">·</span>
  <span class="mono">schiller-offenburg.de</span>
</footer>

<BugButton />
<Celebration />

<style>
  .app-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1.6rem;
    background: var(--sg-petrol);
    color: #fff;
    box-shadow: var(--sg-shadow-sm);
    border-bottom: 3px solid var(--sg-gold);
  }
  .header-left { display: flex; align-items: center; gap: 0.9rem; }
  .brand-mark {
    width: 36px;
    height: 36px;
    border-radius: 8px;
    background: linear-gradient(135deg, var(--sg-gold) 0%, #ffb347 100%);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.2);
    flex-shrink: 0;
  }
  .brand-name {
    font-weight: 700;
    font-size: 1.05rem;
    letter-spacing: 0.01em;
  }
  .brand-tagline {
    font-size: 0.82rem;
    opacity: 0.85;
    margin-top: 0.1rem;
  }
  .header-right { display: flex; align-items: center; gap: 0.8rem; }

  /* Rollen-Badges: gold-betonte Petrol-Varianten */
  :global(.badge.role-fachlehrer) {
    background: var(--sg-success);
    color: #fff;
    border-color: transparent;
  }
  :global(.badge.role-klassenlehrer) {
    background: var(--sg-gold);
    color: var(--sg-petrol);
    border-color: transparent;
    font-weight: 600;
  }
  :global(.badge.role-administrator) {
    background: var(--sg-danger);
    color: #fff;
    border-color: transparent;
  }
  :global(.badge.muted) {
    background: rgba(255, 255, 255, 0.15);
    color: #fff;
    border-color: transparent;
  }

  .btn-logout {
    background: transparent;
    color: #fff;
    border: 1.5px solid rgba(255, 255, 255, 0.4);
    padding: 0.35rem 0.9rem;
    font-size: 0.88rem;
  }
  .btn-logout:hover {
    background: rgba(255, 255, 255, 0.12);
    border-color: #fff;
    box-shadow: none;
  }
  .btn-theme {
    background: transparent;
    color: #fff;
    border: 1.5px solid rgba(255, 255, 255, 0.4);
    padding: 0.3rem 0.7rem;
    font-size: 1rem;
    line-height: 1;
    border-radius: 999px;
  }
  .btn-theme:hover {
    background: rgba(255, 255, 255, 0.12);
    border-color: #fff;
    box-shadow: none;
  }
  .btn-hilfe {
    color: #fff;
    text-decoration: none;
    border: 1.5px solid rgba(255, 255, 255, 0.4);
    padding: 0.32rem 0.8rem;
    font-size: 0.86rem;
    line-height: 1;
    border-radius: 999px;
  }
  .btn-hilfe:hover {
    background: rgba(255, 255, 255, 0.12);
    border-color: #fff;
  }

  .app-main {
    max-width: 1280px;
    margin: 0 auto;
    padding: 2rem 1.6rem;
  }

  .idle-warn {
    position: fixed;
    top: 4.6rem;
    left: 50%;
    transform: translateX(-50%);
    background: #fff8de;
    color: #6b4a00;
    border: 1px solid #d8a000;
    border-radius: 6px;
    padding: 0.55rem 1rem;
    font-size: 0.9rem;
    box-shadow: 0 4px 12px rgba(0,0,0,0.12);
    z-index: 1200;
  }

  .app-footer {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 0.5rem;
    padding: 1.5rem;
    margin-top: 2rem;
    color: var(--sg-meta);
    font-size: 0.82rem;
    border-top: 1px solid var(--sg-border);
  }
  .sep { opacity: 0.5; }

  /* Im Druck-Modus: globalen App-Header + Footer + Bug-Button verstecken,
     nur die Druck-Inhalte (Bogen) bleiben sichtbar. */
  @media print {
    :global(.app-header),
    :global(.app-footer),
    :global(.bug-button) {
      display: none !important;
    }
    :global(.app-main) { padding: 0 !important; margin: 0 !important; }
  }
</style>
