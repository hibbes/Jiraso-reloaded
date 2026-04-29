<!-- src/routes/dashboard/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';
  import { goodies, type Zitat } from '$lib/api';

  let zitat = $state<Zitat | null>(null);

  onMount(async () => {
    if (!session.rolle) {
      goto('/login');
      return;
    }
    try {
      zitat = await goodies.zitat();
    } catch { /* Goodies sind optional, Fehler ignorieren */ }
  });

  type Kachel = {
    titel: string;
    beschreibung: string;
    rollen: string[];
    icon: string;
    tbd?: boolean;
  };

  const kacheln = $derived.by<(Kachel & { href?: string })[]>(() => {
    if (!session.rolle) return [];
    const all: (Kachel & { href?: string })[] = [
      {
        titel: 'Bewertung eingeben',
        beschreibung: 'Matrix Schüler×Kategorie pro Fach, Bemerkung im Detail-Panel',
        rollen: ['fachlehrer', 'klassenlehrer', 'administrator'],
        icon: '📝',
        href: '/bewertung',
      },
      {
        titel: 'Modul-Übersicht',
        beschreibung: 'Alle Bewertungen + Bemerkung pro Schüler:in',
        rollen: ['fachlehrer', 'klassenlehrer', 'administrator'],
        icon: '📊',
        href: '/uebersicht',
      },
      {
        titel: 'Drucken & Export',
        beschreibung: 'Klassen-Bogen drucken oder als PDF speichern',
        rollen: ['klassenlehrer', 'administrator'],
        icon: '🖨️',
        href: '/druck',
      },
      {
        titel: 'Katalog verwalten',
        beschreibung: 'Fächer, Kategorien, Formulierungen',
        rollen: ['administrator'],
        icon: '⚙️',
        href: '/admin/katalog',
      },
      {
        titel: 'Datenverwaltung',
        beschreibung: 'Schüler:innen importieren, Jahreswechsel',
        rollen: ['administrator'],
        icon: '📂',
        tbd: true,
      },
    ];
    return all.filter((k) => k.rollen.includes(session.rolle!));
  });
</script>

<h1>Start</h1>

{#if zitat}
  <blockquote class="zitat-banner">
    <p class="zitat-text">„{zitat.text}"</p>
    <footer class="zitat-autor">— {zitat.autor}</footer>
  </blockquote>
{/if}

<p class="intro text-muted">
  Die Funktionen werden in den folgenden Plänen (2–5) schrittweise aktiviert.
</p>

{#if session.rolle === 'administrator'}
  <a href="/admin/stammdaten" class="admin-tile">
    <strong>Stammdaten-Import</strong>
    <span>XLSX aus ASV-BW einspielen</span>
  </a>
{/if}

<div class="grid">
  {#each kacheln as k}
    {#if k.href}
      <a href={k.href} class="card kachel">
        <div class="kachel-icon" aria-hidden="true">{k.icon}</div>
        <h3>{k.titel}</h3>
        <p class="kachel-desc text-small text-muted">{k.beschreibung}</p>
      </a>
    {:else}
      <div class="card kachel" class:disabled={k.tbd}>
        <div class="kachel-icon" aria-hidden="true">{k.icon}</div>
        <h3>{k.titel}</h3>
        <p class="kachel-desc text-small text-muted">{k.beschreibung}</p>
        {#if k.tbd}
          <span class="badge badge-gold kachel-badge">in Planung</span>
        {/if}
      </div>
    {/if}
  {/each}
</div>

<style>
  .zitat-banner {
    background: linear-gradient(135deg, #f9f5e7 0%, #fff8de 100%);
    border-left: 4px solid var(--sg-petrol, #004058);
    padding: 1rem 1.4rem;
    margin: 0 0 1.6rem;
    border-radius: 6px;
    box-shadow: 0 1px 3px rgba(0,0,0,0.05);
  }
  .zitat-text {
    margin: 0 0 0.4rem;
    font-style: italic;
    color: #2a2a2a;
    font-size: 1.05rem;
    line-height: 1.5;
  }
  .zitat-autor {
    color: #555;
    font-size: 0.9rem;
    text-align: right;
  }
  .intro {
    margin-top: -0.4rem;
    margin-bottom: 2rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
    gap: 1.25rem;
  }
  .kachel {
    display: flex;
    flex-direction: column;
    min-height: 180px;
  }
  .kachel.disabled {
    opacity: 0.85;
  }
  a.kachel {
    text-decoration: none;
    color: inherit;
    transition: box-shadow 0.15s ease;
  }
  a.kachel:hover {
    box-shadow: var(--sg-shadow-hover);
  }
  .kachel-icon {
    font-size: 1.8rem;
    margin-bottom: 0.6rem;
  }
  .kachel h3 {
    margin-bottom: 0.3rem;
  }
  .kachel-desc {
    margin: 0;
    flex: 1;
  }
  .kachel-badge {
    margin-top: 0.9rem;
    align-self: flex-start;
  }
  .admin-tile {
    display: inline-flex;
    flex-direction: column;
    padding: 1rem 1.4rem;
    margin-bottom: 1.5rem;
    background: var(--sg-bg-card);
    border: 1px solid var(--sg-border);
    border-radius: var(--sg-radius-md);
    text-decoration: none;
    color: var(--sg-text);
    box-shadow: var(--sg-shadow-sm);
    transition: box-shadow 0.15s ease;
  }
  .admin-tile:hover {
    box-shadow: var(--sg-shadow-hover);
  }
  .admin-tile strong {
    color: var(--sg-petrol);
  }
  .admin-tile span {
    font-size: 0.9em;
    color: var(--sg-meta);
    margin-top: 0.2rem;
  }
</style>
