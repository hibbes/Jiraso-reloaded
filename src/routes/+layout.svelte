<!-- src/routes/+layout.svelte — Schiller-CI Corporate Design -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { session } from '$lib/session.svelte';
  import { schulname, aktuellesSchuljahr, currentRole, logout } from '$lib/api';
  import { goto } from '$app/navigation';
  import '../app.css';
  import BugButton from '$lib/BugButton.svelte';
  import Celebration from '$lib/Celebration.svelte';

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
  });

  async function handleLogout() {
    await logout();
    session.rolle = null;
    goto('/login');
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
      <button class="btn-logout" onclick={handleLogout}>Abmelden</button>
    {:else}
      <span class="badge muted">nicht angemeldet</span>
    {/if}
  </div>
</header>

<main class="app-main">
  {@render children()}
</main>

<footer class="app-footer">
  <span>Jiraso-reloaded</span>
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
