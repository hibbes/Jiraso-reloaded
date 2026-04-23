<!-- src/routes/dashboard/+page.svelte -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { session } from '$lib/session.svelte';

  onMount(() => {
    if (!session.rolle) goto('/login');
  });

  const kacheln = $derived.by(() => {
    if (!session.rolle) return [];
    const all = [
      { titel: 'Bewertung eingeben', rollen: ['fachlehrer', 'klassenlehrer', 'administrator'] },
      { titel: 'Bemerkung eingeben', rollen: ['klassenlehrer', 'administrator'] },
      { titel: 'Übersicht',          rollen: ['klassenlehrer', 'administrator'] },
      { titel: 'Drucken & Export',   rollen: ['klassenlehrer', 'administrator'] },
      { titel: 'Formulierungen verwalten', rollen: ['administrator'] },
      { titel: 'Datenverwaltung',    rollen: ['administrator'] }
    ];
    return all.filter(k => k.rollen.includes(session.rolle!));
  });
</script>

<h1>Start</h1>
<p class="hint">Die Funktionen werden in den folgenden Plänen (2–5) implementiert.</p>

<div class="grid">
  {#each kacheln as k}
    <div class="kachel">
      <h3>{k.titel}</h3>
      <p class="tbd">(noch nicht implementiert)</p>
    </div>
  {/each}
</div>

<style>
  .hint { color: #666; margin-bottom: 1.5rem; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr)); gap: 1rem; }
  .kachel { background: #f4f4f6; border: 1px solid #d6d6da; border-radius: 4px; padding: 1rem 1.2rem; }
  .kachel h3 { margin: 0 0 0.5rem; font-size: 1.05rem; }
  .tbd { color: #999; font-size: 0.85rem; margin: 0; }
</style>
