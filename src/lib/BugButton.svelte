<script lang="ts">
  import { bugReport, type IssueResponse } from '$lib/api';
  import { celebration } from '$lib/celebration.svelte.ts';

  let offen = $state(false);
  let titel = $state('');
  let beschreibung = $state('');
  let kontext = $state('');
  let submitting = $state(false);
  let issue = $state<IssueResponse | null>(null);
  let fehler = $state<string | null>(null);

  function oeffnen() {
    offen = true;
    issue = null;
    fehler = null;
    titel = '';
    beschreibung = '';
    kontext = `Route: ${window.location.pathname}\nUser-Agent: ${navigator.userAgent}\nZeit: ${new Date().toISOString()}`;
  }
  function schliessen() {
    offen = false;
  }

  async function senden() {
    if (titel.trim().length < 5 || beschreibung.trim().length < 10) {
      fehler = 'Bitte Titel ≥5 Zeichen und Beschreibung ≥10 Zeichen.';
      return;
    }
    submitting = true;
    fehler = null;
    try {
      const body = `${beschreibung}\n\n---\n${kontext}`;
      issue = await bugReport.submit(titel, body);
      celebration.trigger();
    } catch (e) {
      fehler = String(e);
    } finally {
      submitting = false;
    }
  }

  function mailtoFallback(): string {
    const subject = encodeURIComponent(titel || 'Bug in Jiraso-reloaded');
    const body = encodeURIComponent(`${beschreibung}\n\n---\n${kontext}`);
    return `mailto:mczernohous@gmail.com?subject=${subject}&body=${body}`;
  }
</script>

<button class="bug-button" onclick={oeffnen} title="Bug melden">🪲</button>

{#if offen}
  <div class="modal" role="dialog" aria-modal="true">
    <div class="dialog">
      {#if issue}
        <h2>Danke!</h2>
        <p>Issue <a href={issue.html_url} target="_blank" rel="noopener">#{issue.number}</a> wurde angelegt.</p>
        <button onclick={schliessen}>OK</button>
      {:else}
        <h2>Bug melden</h2>
        <label>Titel<input bind:value={titel} placeholder="Kurz: Was ging nicht?" /></label>
        <label>Beschreibung<textarea rows="4" bind:value={beschreibung} placeholder="Was war ich am Tun, was ist passiert?"></textarea></label>
        <label>Kontext (automatisch)<textarea rows="3" bind:value={kontext} readonly></textarea></label>
        {#if fehler}
          <p class="error">{fehler}</p>
          <p>Fallback: <a href={mailtoFallback()}>per E-Mail an Marek</a></p>
        {/if}
        <div class="actions">
          <button onclick={schliessen}>Abbrechen</button>
          <button onclick={senden} disabled={submitting}>{submitting ? 'Sende...' : 'Senden'}</button>
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  .bug-button {
    position: fixed;
    bottom: 1rem;
    right: 1rem;
    width: 3rem;
    height: 3rem;
    border-radius: 50%;
    border: 0;
    background: var(--sg-petrol, #004058);
    color: white;
    font-size: 1.4rem;
    cursor: pointer;
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
    z-index: 9990;
  }
  .bug-button:hover { transform: scale(1.05); }
  .modal {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.4);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9991;
  }
  .dialog {
    background: white;
    padding: 1.5rem;
    border-radius: 8px;
    width: min(500px, 90vw);
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .dialog label { display: flex; flex-direction: column; gap: 0.2rem; font-size: 0.9rem; }
  .dialog input, .dialog textarea {
    padding: 0.4rem;
    font-family: inherit;
    border: 1px solid #ccc;
    border-radius: 4px;
  }
  .dialog .error { color: #c00; }
  .dialog .actions { display: flex; gap: 0.5rem; justify-content: flex-end; }
</style>
