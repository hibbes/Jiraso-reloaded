<!-- src/routes/hilfe/+page.svelte
     Anleitung fuer Kolleg:innen am Schiller-Gymnasium.
     Ueber Header-Knopf "? Hilfe" jederzeit erreichbar.
     Inhalt bewusst ausfuehrlich, mit Lesefluss von Schnellstart -> Tiefe. -->
<script lang="ts">
  import { session } from '$lib/session.svelte';

  type Section = { id: string; titel: string };
  const sections: Section[] = [
    { id: 'schnellstart',     titel: 'Schnellstart (3 Schritte)' },
    { id: 'login',            titel: 'Anmelden — welche Rolle bin ich?' },
    { id: 'bewertung',        titel: 'Bewertungen eingeben' },
    { id: 'tastatur',         titel: 'Tastatur-Workflow (Panel-Modus)' },
    { id: 'bemerkung',        titel: 'Bemerkung schreiben (nur Klassenlehrer:in)' },
    { id: 'kuerzel',          titel: 'Mein Kürzel — was ist das?' },
    { id: 'konflikt',         titel: 'Konflikt: zwei haben gleichzeitig editiert' },
    { id: 'uebersicht',       titel: 'Wer ist schon fertig? (Übersicht)' },
    { id: 'drucken',          titel: 'Drucken & PDF speichern' },
    { id: 'kaefer',           titel: 'Etwas funktioniert nicht — Käfer-Knopf 🪲' },
    { id: 'troubleshooting',  titel: 'Troubleshooting' },
    { id: 'admin',            titel: 'Admin-Aufgaben (nur Schul-Admin)' },
  ];
</script>

<main class="container">
  <header>
    <h1>Hilfe & Anleitung</h1>
    <a href="/dashboard">← Dashboard</a>
  </header>

  <p class="lead">
    Jiraso-reloaded ist die Verbalbeurteilungs-Software für die Klassen 5/6.
    Diese Anleitung beantwortet die Fragen, die im Kollegium am häufigsten
    aufkommen. Bei Problemen, die hier nicht stehen: <a href="#kaefer">Käfer-Knopf</a>
    rechts unten.
  </p>

  <nav class="toc" aria-label="Inhaltsverzeichnis">
    <strong>Inhalt</strong>
    <ol>
      {#each sections as s (s.id)}
        <li><a href={`#${s.id}`}>{s.titel}</a></li>
      {/each}
    </ol>
  </nav>

  <article id="schnellstart">
    <h2>1. Schnellstart (3 Schritte)</h2>
    <ol class="steps">
      <li>
        <strong>App starten</strong> — Doppelklick auf die Desktop-Verknüpfung
        „Jiraso". Die App startet vom Schul-Netzlaufwerk, die Daten liegen
        zentral und werden automatisch zwischen allen PCs geteilt.
      </li>
      <li>
        <strong>Anmelden</strong> — Rolle wählen (Fachlehrer:in / Klassenlehrer:in /
        Admin). Fachlehrer:innen kommen direkt rein, ohne Passwort.
        Klassenlehrer:innen + Admin haben jeweils ein Passwort, das die
        Schul-Admin verteilt.
      </li>
      <li>
        <strong>Bewerten</strong> — auf dem Dashboard die Kachel
        „Bewertung eingeben" anklicken. Klasse + Fach wählen. Pro Schüler:in
        7 Klicks (oder 7 Tastendrücke), fertig.
      </li>
    </ol>
  </article>

  <article id="login">
    <h2>2. Anmelden — welche Rolle bin ich?</h2>
    <p>Es gibt drei Rollen mit unterschiedlichen Rechten:</p>
    <table class="rolle-tabelle">
      <thead>
        <tr><th>Rolle</th><th>Wer ist das?</th><th>Was darf die Rolle?</th></tr>
      </thead>
      <tbody>
        <tr>
          <td><span class="badge role-fachlehrer">Fachlehrer:in</span></td>
          <td>Alle Lehrkräfte, die ein Fach in einer 5./6. Klasse unterrichten</td>
          <td>Bewertungen für die eigenen Fächer einsetzen. Kein Passwort
              nötig — der Schul-PC ist physisch geschützt, ein zusätzliches
              App-Passwort wäre nur Reibung.</td>
        </tr>
        <tr>
          <td><span class="badge role-klassenlehrer">Klassenlehrer:in</span></td>
          <td>Eine Lehrkraft pro Klasse</td>
          <td>Alles, was Fachlehrer:in darf, plus die persönliche Bemerkung
              („zusätzliche Bemerkungen") pro Schüler:in schreiben.</td>
        </tr>
        <tr>
          <td><span class="badge role-administrator">Administrator</span></td>
          <td>Schul-Admin (1–2 Personen)</td>
          <td>Stammdaten-Import (Klassen + Schüler:innen aus ASV-BW),
              Katalog-Verwaltung (Fächer + Floskeln), alle Lehrer-Funktionen.</td>
        </tr>
      </tbody>
    </table>
    <p>
      Im Login-Bildschirm einfach die eigene Rolle anklicken. Bei
      Klassenlehrer:in / Admin folgt das Passwort-Feld.
    </p>
    <p class="hinweis">
      <strong>Falsches Passwort?</strong> Bei Klassenlehrer:innen / Admins
      kann sich nur die Schul-Admin um neue Passwörter kümmern (siehe
      <a href="#admin">Admin-Aufgaben</a>).
    </p>
  </article>

  <article id="bewertung">
    <h2>3. Bewertungen eingeben</h2>
    <p>Dashboard → Kachel <strong>„Bewertung eingeben"</strong>.</p>
    <ol>
      <li><strong>Klasse</strong> wählen (z.&nbsp;B. „5a")</li>
      <li><strong>Fach</strong> wählen (z.&nbsp;B. „Mathematik")</li>
      <li><strong>Kürzel</strong> eintragen, falls noch nicht geschehen
          (siehe <a href="#kuerzel">Mein Kürzel</a>)</li>
    </ol>
    <p>
      Es gibt zwei Ansichten — oben rechts umschaltbar:
    </p>
    <dl class="zwei-ansichten">
      <dt>Panel-Modus (Default)</dt>
      <dd>
        Links die Schüler:innen-Liste mit Fortschritts-Bullets
        (○ noch nichts · ◐ angefangen · ● fertig). Rechts pro Schüler:in
        alle 7 Kategorien als Karten untereinander, jede mit 3–5
        Floskeln als farbige „Pills" (grün → gelb → rot).
        <strong>Klick auf eine Pill speichert sofort</strong> und springt
        automatisch zur nächsten Kategorie. Nach Kategorie 7 geht es
        direkt zur nächsten Schüler:in (Kategorie 1).
      </dd>
      <dt>Matrix-Modus</dt>
      <dd>
        Klassische Tabelle: Schüler:innen-Zeilen × Kategorie-Spalten.
        Pro Zelle ein Drop-Down. Eher Übersichts-Ansicht; für die
        eigentliche Eingabe ist der Panel-Modus schneller.
      </dd>
    </dl>
    <p>
      <strong>Speichern passiert automatisch</strong> bei jedem Klick. Du musst
      nichts „abschicken". Ein kleines ⟳ (speichere…) wechselt zu ✓ (gespeichert)
      sobald die Zeile auf der Festplatte liegt.
    </p>
  </article>

  <article id="tastatur">
    <h2>4. Tastatur-Workflow (Panel-Modus)</h2>
    <p>
      Im Panel-Modus reicht <strong>die Tastatur</strong>, sobald die richtige
      Schüler:in fokussiert ist. Sieben Tastendrücke pro Schüler:in:
    </p>
    <table class="taste-tabelle">
      <thead><tr><th>Taste</th><th>Aktion</th></tr></thead>
      <tbody>
        <tr><td><kbd>1</kbd>–<kbd>5</kbd></td><td>Floskel in der aktiven Kategorie wählen</td></tr>
        <tr><td><kbd>0</kbd> / <kbd>-</kbd></td><td>„keine Angabe"</td></tr>
        <tr><td><kbd>↑</kbd> / <kbd>↓</kbd> oder <kbd>n</kbd> / <kbd>p</kbd></td><td>nächste / vorherige Schüler:in</td></tr>
        <tr><td><kbd>←</kbd> / <kbd>→</kbd></td><td>Kategorie wechseln, ohne etwas zu wählen</td></tr>
        <tr><td><kbd>m</kbd></td><td>Panel ⇄ Matrix</td></tr>
        <tr><td><kbd>a</kbd></td><td>Auto-Advance an / aus (springt nach jeder Auswahl weiter)</td></tr>
        <tr><td><kbd>?</kbd></td><td>Cheat-Sheet einblenden</td></tr>
        <tr><td><kbd>Esc</kbd></td><td>Bemerkungs-Feld verlassen / Cheat-Sheet schließen</td></tr>
      </tbody>
    </table>
    <p>
      Das Cheat-Sheet ist im Panel-Modus jederzeit über den Knopf „? Tastatur"
      oben erreichbar.
    </p>
  </article>

  <article id="bemerkung">
    <h2>5. Bemerkung schreiben (nur Klassenlehrer:in)</h2>
    <p>
      Unter den 7 Kategorie-Karten gibt es bei Klassenlehrer:innen + Admins
      ein zusätzliches Bemerkungs-Feld („zusätzliche Bemerkungen", max
      700 Zeichen).
    </p>
    <ul>
      <li><strong>Cursor ins Feld</strong> nur per Maus-Klick — automatisch
          springt der Fokus dort nicht hin, damit Tastatur-Workflow nicht
          unterbrochen wird.</li>
      <li><strong>Speichern</strong> automatisch 1,5&nbsp;Sekunden nach der
          letzten Tastatureingabe oder beim Verlassen des Feldes (<kbd>Esc</kbd>
          oder Klick außerhalb).</li>
      <li><strong>Counter</strong> zeigt verbrauchte / 700 Zeichen, wird
          rot ab 700.</li>
      <li>Fachlehrer:innen sehen die Bemerkung nicht — das ist gewollt.</li>
    </ul>
  </article>

  <article id="kuerzel">
    <h2>6. Mein Kürzel — was ist das?</h2>
    <p>
      Beim ersten Bewerten fragt die App nach deinem <strong>Kürzel</strong>
      (z.&nbsp;B. „MZ" oder „CZE", 2–8 Zeichen). Es wird an jede Bewertung
      gehängt — andere Lehrkräfte sehen dann, dass du der letzte Editor warst.
    </p>
    <p>
      <strong>Sinn:</strong> Wenn du in der falschen Klasse oder im falschen
      Fach landest, warnt dich die App, bevor du fremde Bewertungen
      überschreibst. Ohne Kürzel ginge das verloren.
    </p>
    <p>
      Das Kürzel kannst du später jederzeit per Klick auf das Kürzel-Pill
      oben rechts in der Bewertungs-Maske ändern. Es bleibt pro Browser
      gespeichert.
    </p>
  </article>

  <article id="konflikt">
    <h2>7. Konflikt: zwei haben gleichzeitig editiert</h2>
    <p>
      Wenn zwei Lehrkräfte die <em>gleiche</em> Zelle (Schüler × Fach × Kategorie)
      gleichzeitig editieren, erkennt die App das beim Speichern. Es erscheint
      ein <strong>Konflikt-Banner</strong> mit zwei Knöpfen:
    </p>
    <ul>
      <li><strong>Meine</strong> — deine Eingabe gewinnt, die Eingabe
          der anderen Lehrkraft wird überschrieben.</li>
      <li><strong>Ihre</strong> — du behältst die Eingabe der anderen
          Lehrkraft, deine wird verworfen.</li>
    </ul>
    <p>
      Im Zweifelsfall lieber <strong>kurz nachfragen</strong>, bevor du auf
      „Meine" klickst — die andere Lehrkraft sieht ihr Kürzel im Banner.
    </p>
  </article>

  <article id="uebersicht">
    <h2>8. Wer ist schon fertig? (Übersicht)</h2>
    <p>Dashboard → Kachel <strong>„Modul-Übersicht"</strong>. Zwei Sichten:</p>
    <ul>
      <li><strong>Pro Schüler:in</strong>: Liste aller Schüler:innen einer
          Klasse, Klick auf einen Namen zeigt das vollständige
          Beurteilungs-Bild.</li>
      <li><strong>Vollständigkeit (Klasse × Fach)</strong>: Matrix mit
          Fach-Zeilen × Kategorie-Spalten. Grün = vollständig, gelb =
          angefangen, rot = leer. Auf einen Blick sichtbar, wo noch
          Lücken sind.</li>
    </ul>
  </article>

  <article id="drucken">
    <h2>9. Drucken &amp; PDF speichern</h2>
    <p>Dashboard → Kachel <strong>„Drucken &amp; Export"</strong>. Schritt für Schritt:</p>
    <ol>
      <li>Klasse wählen.</li>
      <li>Datum eintragen (steht später unten auf jedem Bogen).</li>
      <li>Aus der Liste die zu druckenden Schüler:innen anhaken
          („Alle / Keine / Nur Fertige" als Schnell-Aktionen).</li>
      <li>„📄 Klasse als ein PDF speichern" klicken.</li>
      <li>Im erscheinenden Druck-Dialog als Drucker
          <strong>„Microsoft Print to PDF"</strong> oder
          <strong>„Als PDF speichern"</strong> wählen.</li>
      <li>Speicherort + Dateinamen wählen — alle ausgewählten Bögen
          landen in <em>einer</em> PDF-Datei (eine A4-Seite pro Schüler:in).</li>
    </ol>
    <p>
      Für <strong>echten Papier-Druck</strong> einfach im gleichen Dialog
      den normalen Drucker statt „Microsoft Print to PDF" wählen.
    </p>
    <p class="hinweis">
      Das Layout entspricht dem alten Schiller-Original (12 Fächer als
      vertikale Spalten, 27 Floskeln als Zeilen, X-Markierungen, Bemerkungs-
      Kasten, Datum + Unterschriftsfelder).
    </p>
  </article>

  <article id="kaefer">
    <h2>10. Etwas funktioniert nicht — Käfer-Knopf 🪲</h2>
    <p>
      Rechts unten in jeder Maske gibt es einen kleinen Käfer-Knopf. Klick
      drauf öffnet ein Mini-Formular mit:
    </p>
    <ul>
      <li><strong>Titel</strong> — eine Zeile, was kaputt ist</li>
      <li><strong>Beschreibung</strong> — was ist passiert, was hattest du
          erwartet, was passiert stattdessen</li>
      <li><strong>Kontext</strong> (optional) — auf welcher Seite, mit
          welcher Klasse / welchem Fach</li>
    </ul>
    <p>
      Beim Absenden wird automatisch ein <strong>GitHub-Issue</strong> im
      Jiraso-Repo eröffnet. Die Schul-Admin und Marek bekommen das mit
      und kümmern sich.
    </p>
    <p>
      Falls keine GitHub-Verbindung konfiguriert ist, fällt der Knopf auf
      einen <strong>mailto-Link</strong> zurück — Mail an
      <code>mczernohous@gmail.com</code>.
    </p>
  </article>

  <article id="troubleshooting">
    <h2>11. Troubleshooting</h2>
    <dl class="trouble">
      <dt>„App ist belegt — Login nicht möglich"</dt>
      <dd>
        Maximal 2 PCs dürfen gleichzeitig eingeloggt sein. Wenn du als 3.
        einloggen willst, siehst du die Namen der beiden anderen
        eingeloggten Geräte. Lösung: an einem der beiden anderen PCs
        abmelden — dann ist der Slot frei. Wenn niemand mehr aktiv ist
        (App-Crash o.&nbsp;ä.), wird der „verwaiste" Slot nach 8 Stunden
        automatisch freigegeben. Wer es eilig hat: Admin kann den Lock
        manuell zurücksetzen (Knopf „Lock zurücksetzen" auf der
        Login-Seite).
      </dd>

      <dt>„Bewertung wird nicht gespeichert (⟳ bleibt stehen)"</dt>
      <dd>
        Wahrscheinlich Netzwerk-Aussetzer auf dem Schul-Laufwerk. App
        kurz schließen + neu starten, die letzte Eingabe ist im Zweifel
        weg und muss neu gemacht werden. Falls dauerhaft: Schul-Admin
        ansprechen.
      </dd>

      <dt>„Ich sehe Klasse X nicht in der Liste"</dt>
      <dd>
        Die Klasse muss erst von der Schul-Admin importiert sein
        (ASV-BW-XLSX → /admin/stammdaten). Nach Import steht sie sofort
        allen Lehrkräften zur Verfügung.
      </dd>

      <dt>„Druck-Vorschau zeigt komische Zeilenumbrüche"</dt>
      <dd>
        Browser-Druck-Dialog → Layout-Einstellung „Hochformat" + Skalierung
        „100%" + Ränder „Standard". Format ist auf A4 hochkant ausgelegt.
      </dd>
    </dl>
  </article>

  {#if session.rolle === 'administrator'}
    <article id="admin">
      <h2>12. Admin-Aufgaben (nur Schul-Admin)</h2>
      <p>Drei Bereiche, die nur du als Admin siehst:</p>
      <h3>Stammdaten-Import</h3>
      <p>
        <code>/admin/stammdaten</code> — XLSX/CSV-Export aus ASV-BW
        einspielen. Spalten werden heuristisch erkannt, ASV-UUID dient als
        Stable-Key (Re-Import einer Klasse ändert nur was sich geändert hat).
        Schuljahr-Verwaltung im gleichen Wizard.
      </p>
      <h3>Katalog verwalten</h3>
      <p>
        <code>/admin/katalog</code> — drei Tabs:
      </p>
      <ul>
        <li><strong>Fächer</strong>: Reihenfolge ändern, einzelne deaktivieren
            (z.&nbsp;B. wenn ein Fach in einer Klassenstufe nicht angeboten
            wird).</li>
        <li><strong>Kategorien</strong>: die 7 Beurteilungs-Kategorien
            (Lernbereitschaft, Auffassungsgabe, …). Reihenfolge ändern,
            inaktivieren — Vorsicht, wirkt sich auf alle Bewertungen aus.</li>
        <li><strong>Floskeln</strong>: pro Kategorie die 3–5
            Formulierungen. Neue hinzufügen, alte deaktivieren.</li>
      </ul>
      <p>
        Beim Erst-Setup: „Default-Fächer-Seed" + „Default-Floskeln-Seed"
        einmal anklicken — dann steht das Original-Schiller-Set sofort drin
        (12 Fächer, 7 Kategorien, 27 Formulierungen).
      </p>
      <p>
        Alternativ: Legacy-Import unter <code>/admin/legacy-import</code> —
        importiert <code>Fächer.txt</code>, <code>Floskeln.txt</code>,
        <code>format.xls</code> aus dem alten Schiller-Programmpaket.
      </p>
      <h3>Test-Daten würfeln</h3>
      <p>
        Im Bewertungs-Bildschirm hat der Admin einen 🎲-Knopf zum
        Schnell-Bewerten einer kompletten Klasse mit zufälligen
        Werten — nur zum Testen, in echter Klasse nie verwenden!
      </p>
      <h3>Passwörter</h3>
      <p>
        Das Klassenlehrer- und Admin-Passwort werden bei der Erst-Einrichtung
        gesetzt (Setup-Bildschirm). Zum Ändern: in <code>config.toml</code>
        neben der jiraso-reloaded.exe das jeweilige Hash-Feld leeren — beim
        nächsten Start läuft der Setup-Wizard erneut.
      </p>
    </article>
  {/if}

  <p class="footer-note">
    Fragen, die hier nicht stehen? Käfer-Knopf rechts unten oder
    direkt an Marek (<code>mczernohous@gmail.com</code>).
  </p>
</main>

<style>
  .container {
    max-width: 850px;
    margin: 0 auto;
    padding: 2rem 1rem 4rem;
  }
  header { display: flex; justify-content: space-between; align-items: center; }
  .lead {
    font-size: 1.05rem;
    color: var(--sg-text);
    margin: 1rem 0 1.5rem;
    line-height: 1.5;
  }
  .toc {
    background: var(--sg-bg-card);
    border: 1px solid var(--sg-border);
    border-radius: 6px;
    padding: 0.9rem 1.2rem;
    margin-bottom: 2rem;
  }
  .toc strong { display: block; margin-bottom: 0.4rem; color: var(--sg-petrol); }
  .toc ol { margin: 0; padding-left: 1.4rem; line-height: 1.7; }
  .toc a { color: var(--sg-text); text-decoration: none; }
  .toc a:hover { text-decoration: underline; color: var(--sg-petrol); }

  article {
    margin-bottom: 2.4rem;
    scroll-margin-top: 80px;
  }
  article h2 {
    color: var(--sg-petrol);
    border-bottom: 2px solid var(--sg-gold);
    padding-bottom: 0.3rem;
    margin-bottom: 0.8rem;
  }
  article h3 {
    color: var(--sg-petrol);
    margin: 1.2rem 0 0.4rem;
    font-size: 1.05rem;
  }
  article p, article li, article dd { line-height: 1.55; }
  article ol, article ul { padding-left: 1.4rem; }
  article ol li, article ul li { margin-bottom: 0.4rem; }

  .steps li { margin-bottom: 0.7rem; }

  .hinweis {
    background: var(--sg-bg-card);
    border-left: 3px solid var(--sg-gold);
    padding: 0.6rem 0.9rem;
    border-radius: 4px;
  }

  .rolle-tabelle, .taste-tabelle {
    border-collapse: collapse;
    width: 100%;
    margin: 0.8rem 0;
    font-size: 0.92rem;
  }
  .rolle-tabelle th, .rolle-tabelle td,
  .taste-tabelle th, .taste-tabelle td {
    border: 1px solid var(--sg-border);
    padding: 0.5rem 0.7rem;
    vertical-align: top;
    text-align: left;
  }
  .rolle-tabelle th, .taste-tabelle th {
    background: var(--sg-bg-card);
    color: var(--sg-petrol);
    font-size: 0.85rem;
  }

  .zwei-ansichten { margin: 0.6rem 0; }
  .zwei-ansichten dt {
    font-weight: 600;
    margin-top: 0.6rem;
    color: var(--sg-petrol);
  }
  .zwei-ansichten dd {
    margin: 0.2rem 0 0 0;
    padding-left: 0.7rem;
    border-left: 2px solid var(--sg-border);
  }

  .trouble dt {
    font-weight: 600;
    margin-top: 0.9rem;
    color: var(--sg-petrol);
  }
  .trouble dd {
    margin: 0.2rem 0 0;
  }

  kbd {
    display: inline-block;
    padding: 0.05rem 0.4rem;
    border: 1px solid var(--sg-border);
    border-bottom-width: 2px;
    border-radius: 3px;
    background: var(--sg-bg-card);
    font-family: ui-monospace, monospace;
    font-size: 0.85em;
    line-height: 1.2;
  }
  code {
    background: var(--sg-bg-card);
    padding: 0.05rem 0.35rem;
    border-radius: 3px;
    font-family: ui-monospace, monospace;
    font-size: 0.92em;
  }

  .footer-note {
    margin-top: 3rem;
    padding-top: 1rem;
    border-top: 1px solid var(--sg-border);
    color: var(--sg-meta);
    font-size: 0.9rem;
    text-align: center;
  }
</style>
