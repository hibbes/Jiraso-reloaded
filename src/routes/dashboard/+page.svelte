<!-- src/routes/dashboard/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';

  onMount(() => {
    if (!session.rolle) goto('/login');
  });

  type Kachel = {
    titel: string;
    beschreibung: string;
    rollen: string[];
    icon: string;
    tbd?: boolean;
  };

  const kacheln = $derived.by<Kachel[]>(() => {
    if (!session.rolle) return [];
    const all: Kachel[] = [
      {
        titel: 'Bewertung eingeben',
        beschreibung: '7 Kategorien pro Fach, Ziffern 0–4',
        rollen: ['fachlehrer', 'klassenlehrer', 'administrator'],
        icon: '📝',
        tbd: true,
      },
      {
        titel: 'Bemerkung eingeben',
        beschreibung: 'Individuelle Verbalbeurteilung je Schüler:in',
        rollen: ['klassenlehrer', 'administrator'],
        icon: '💬',
        tbd: true,
      },
      {
        titel: 'Übersicht',
        beschreibung: 'Klasse im Blick, Bewertungen nachverfolgen',
        rollen: ['klassenlehrer', 'administrator'],
        icon: '📊',
        tbd: true,
      },
      {
        titel: 'Drucken & Export',
        beschreibung: 'PDF für Eltern-Rückmeldung, A4 quer',
        rollen: ['klassenlehrer', 'administrator'],
        icon: '🖨️',
        tbd: true,
      },
      {
        titel: 'Formulierungen verwalten',
        beschreibung: 'Fächer, Kategorien, Standard-Floskeln',
        rollen: ['administrator'],
        icon: '⚙️',
        tbd: true,
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
<p class="intro text-muted">
  Die Funktionen werden in den folgenden Plänen (2–5) schrittweise aktiviert.
</p>

<div class="grid">
  {#each kacheln as k}
    <div class="card kachel" class:disabled={k.tbd}>
      <div class="kachel-icon" aria-hidden="true">{k.icon}</div>
      <h3>{k.titel}</h3>
      <p class="kachel-desc text-small text-muted">{k.beschreibung}</p>
      {#if k.tbd}
        <span class="badge badge-gold kachel-badge">in Planung</span>
      {/if}
    </div>
  {/each}
</div>

<style>
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
</style>
