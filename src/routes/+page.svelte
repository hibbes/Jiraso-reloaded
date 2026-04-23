<!-- src/routes/+page.svelte — Root redirect je nach Session/Setup-Zustand -->
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { needsSetup, currentRole } from '$lib/api';
  import { session } from '$lib/session.svelte';

  onMount(async () => {
    if (await needsSetup()) {
      goto('/setup');
      return;
    }
    const rolle = await currentRole();
    session.rolle = rolle;
    goto(rolle ? '/dashboard' : '/login');
  });
</script>

<p>Lade…</p>
