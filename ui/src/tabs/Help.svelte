<script lang="ts">
  import { defaultHelpSection, helpSections, type HelpId } from '../lib/help';

  let open = $state<HelpId>(defaultHelpSection);
</script>

<h1>Aide</h1>

<div class="layout">
  <ul class="toc">
    {#each helpSections as s (s.id)}
      <li><button class:active={open === s.id} onclick={() => (open = s.id)}>{s.title}</button></li>
    {/each}
  </ul>

  <section class="card body">
    {#if open === 'start'}
      <h2>Premiers pas</h2>
      <ol>
        <li>Onglet <b>Installation</b> : vérifiez que tout est là (whisper-server, un modèle Whisper, et si vous voulez une mise au propre sur cet ordinateur, llama-server et un modèle). La taille et la source sont montrées avant tout téléchargement.</li>
        <li>Onglet <b>Banc d’essai</b> : lisez une phrase à voix haute pour vérifier le micro et voir ce que Whisper entend.</li>
        <li>Dans n’importe quelle appli : <b>maintenez Ctrl droit</b>, parlez, relâchez. Le texte est collé.</li>
      </ol>
    {:else if open === 'dictate'}
      <h2>Dicter et traduire</h2>
      <ul>
        <li><b>Ctrl droit</b> maintenu : dictée. Relâcher : transcription, mise au propre et collage.</li>
        <li><b>Maj</b> en plus pendant l’appui : le texte est <b>traduit</b> (langue choisie dans Réglages).</li>
        <li>Une <b>autre touche</b> pendant l’appui annule : rien n’est collé.</li>
        <li>Dites la ponctuation : « virgule », « point », « point d’interrogation », « à la ligne ».</li>
      </ul>
    {:else if open === 'modes'}
      <h2>Les modes</h2>
      <p>Le mode règle le ton de la mise au propre : <b>Littéral</b> (aucun modèle), <b>Simple</b> (retire les « euh », met la ponctuation), <b>Soutenu</b> et les modes métiers. Vous pouvez modifier la consigne d’un mode ou créer le vôtre, avec des exemples : ce sont eux qui donnent le style. Sans moteur de mise au propre, seul le mode Littéral s’applique.</p>
    {:else if open === 'dictionary'}
      <h2>Dictionnaire et signes</h2>
      <p>Les <b>mots</b> du dictionnaire sont écrits exactement comme vous les avez saisis. Une <b>correction</b> « entendu → écrit » remplace toujours ce que Whisper entend. Les signes dictés (« ouvrez la parenthèse », « tiret » pour une liste) marchent sans rien configurer.</p>
    {:else if open === 'indicator'}
      <h2>L’indicateur</h2>
      <p>L’icône près de l’horloge change de couleur : <b>gris</b> au repos, <b>rouge</b> pendant l’écoute, <b>orange</b> pendant le traitement, <b>vert</b> une fois le texte collé, <b>bleu</b> pour un message. Une pastille à l’écran viendra plus tard.</p>
    {:else if open === 'measures'}
      <h2>Mesurer et comparer</h2>
      <p>Le <b>Banc d’essai</b> fait lire les 17 phrases de Scribe et contrôle automatiquement le résultat (chiffres, ponctuation, mots gardés) en donnant les temps de chaque étape. L’onglet <b>Journal et rapport</b> copie un rapport à coller dans la conversation. Sans carte graphique, comptez 2 à 3 s de transcription ; une mise au propre dans le cloud ajoute 1 s, sur cet ordinateur 2 à 4 s.</p>
    {:else if open === 'privacy'}
      <h2>Sur cet ordinateur ou en ligne</h2>
      <p>Avec <b>Whisper</b> et <b>llama.cpp</b>, rien ne quitte l’ordinateur. <b>Deepgram</b> (transcription) et <b>OpenRouter</b> (mise au propre) envoient le son ou le texte à leur service : c’est un dépannage quand l’ordinateur est trop lent. Les clés sont gardées dans le Gestionnaire d’identifiants de Windows. L’historique reste sur cet ordinateur ; vous réglez sa durée dans l’onglet Historique.</p>
    {:else}
      <h2>Problèmes fréquents</h2>
      <ul>
        <li><b>Rien n’est collé</b> : lisez le journal ; le collage est bloqué dans une fenêtre lancée en administrateur.</li>
        <li><b>Micro faible ou muet</b> : mesurez-le dans le Banc d’essai ; réglez le volume d’entrée de Windows.</li>
        <li><b>La première dictée est très lente</b> : le modèle local se recharge après une libération ; réglez « Garder le modèle chargé » dans Réglages.</li>
        <li><b>Il manque un programme ou un modèle</b> : onglet Installation.</li>
        <li><b>Mise à jour</b> : onglet « Mise à jour », puis « Chercher une nouvelle version » et « Installer et redémarrer ».</li>
      </ul>
    {/if}
  </section>
</div>

<style>
  .layout { display: flex; gap: 14px; align-items: flex-start; flex-wrap: wrap; }
  .toc { list-style: none; margin: 0; padding: 0; width: 220px; flex: none; display: flex; flex-direction: column; gap: 2px; }
  .toc button { width: 100%; text-align: left; border: 0; background: transparent; padding: 8px 10px; border-radius: 8px; }
  .toc button:hover { background: var(--line); }
  .toc button.active { background: var(--accent); color: var(--accent-ink); }
  .body { flex: 1 1 360px; margin: 0; }
</style>
