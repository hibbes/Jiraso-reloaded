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
    href: string;
  };

  const kacheln = $derived.by<Kachel[]>(() => {
    if (!session.rolle) return [];
    const all: Kachel[] = [
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
        titel: 'Stammdaten-Import',
        beschreibung: 'Schüler:innen aus ASV-BW (XLSX/CSV), Schuljahr-Verwaltung',
        rollen: ['administrator'],
        icon: '📂',
        href: '/admin/stammdaten',
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

<div class="grid">
  {#each kacheln as k}
    <a href={k.href} class="card kachel">
      <div class="kachel-icon" aria-hidden="true">{k.icon}</div>
      <h3>{k.titel}</h3>
      <p class="kachel-desc text-small text-muted">{k.beschreibung}</p>
    </a>
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
</style>
