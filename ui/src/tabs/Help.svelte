<script lang="ts">
  import { onMount } from 'svelte';
  import OverlayView, { type View } from '../overlay/OverlayView.svelte';
  import { helpSections } from '../lib/help';
  import { nav } from '../lib/nav.svelte';
  import { settings } from '../lib/settings.svelte';
  import { status } from '../lib/status.svelte';
  import type { TabId } from '../lib/tabs';

  let s = $derived(status.value);
  let v = $derived(settings.value);
  let open = $derived(nav.helpSection);
  /** Un clic sur une section : on repart du haut (sinon une section courte s'ouvre avec son titre hors de vue). */
  function pick(id: typeof nav.helpSection) {
    nav.helpSection = id;
    window.scrollTo(0, 0);
  }
  /** Les touches en vigueur (réglables dans Réglages → Dictée). */
  /** Le système sur lequel tourne Scriibe : l’Aide ne parle que de lui (touches, icône, autorisations, trousseau). */
  let os = $derived(status.os);
  let mac = $derived(os === 'macos');
  let linux = $derived(os === 'linux');
  let defaultDictate = $derived(mac ? 'Option droite' : 'Ctrl droit');
  let defaultTranslate = $derived(mac ? '⌘' : 'Maj');
  let copyKey = $derived(mac ? '⌘C' : 'Ctrl+C');
  let pasteKey = $derived(mac ? '⌘V' : 'Ctrl+V');
  let trayNear = $derived(mac ? 'dans la barre des menus' : 'près de l’horloge');
  let trayIcon = $derived(mac ? 'L’icône de la barre des menus' : 'L’icône près de l’horloge');
  let editor = $derived(mac ? 'TextEdit' : linux ? 'un éditeur de texte' : 'le Bloc-notes');
  let pasteNote = $derived(
    mac
      ? 'Le plus souvent, c’est l’autorisation « Contrôle de l’appareil et accès aux données » (« Accessibilité » selon la version de macOS) qui manque, dans Réglages Système → Confidentialité et sécurité : active l’interrupteur de Scriibe, puis quitte Scriibe et relance-le.'
      : linux
        ? 'Dans un terminal, c’est Ctrl+Maj+V qui colle.'
        : 'Le collage est bloqué par Windows dans une fenêtre lancée en administrateur.',
  );
  let dictate = $derived(s?.dictation_label ?? defaultDictate);
  let translate = $derived(s?.translate_label ?? defaultTranslate);
  let language = $derived(
    ({ en: 'anglais', es: 'espagnol', de: 'allemand', it: 'italien', pt: 'portugais (Brésil)' } as Record<string, string>)[v?.translationLanguage ?? 'en'] ?? 'anglais',
  );

  onMount(() => void settings.load());

  /** Une phrase de la page sur ce qui part ou non sur Internet, d'après les moteurs choisis. */
  let privacySummary = $derived.by(() => {
    if (!v) return '';
    const voice = v.transcriptionEngine === 'whisper' ? 'ta voix reste sur cet ordinateur' : 'l’enregistrement de chaque dictée part chez Deepgram';
    const local = v.cleanupEngine === 'llamaCpp' || v.cleanupEngine === 'lmStudio' || v.cleanupEngine === 'ollama';
    const text =
      v.cleanupEngine === 'none'
        ? 'aucun modèle ne met le texte au propre (la traduction est alors indisponible)'
        : local
          ? 'le texte est mis au propre sur cet ordinateur'
          : 'le texte part chez OpenRouter pour être mis au propre';
    const everythingLocal = v.transcriptionEngine === 'whisper' && local;
    return `Avec tes réglages actuels, ${voice}, et ${text}.${everythingLocal ? ' Ta voix et ton texte restent sur cet ordinateur.' : ''}`;
  });

  /** L'indicateur dans ses trois états, avec le style, la couleur et la transparence réglés. */
  function sample(state: View['state']): View {
    return {
      visible: true,
      state,
      message: null,
      translation: null,
      mic: 'ok',
      level: 0.6,
      appearance: { enabled: true, style: v?.captureStyle ?? 'pill', position: 'center', color: v?.captureColor ?? 'scribe', opacity: v?.captureOpacity ?? 1 },
    };
  }
  const curve = Array.from({ length: 40 }, (_, i) => Math.max(0.1, 0.35 + 0.3 * Math.sin(i / 3) + 0.2 * Math.sin(i / 1.3)));
</script>

{#snippet go(label: string, tab: TabId, anchor?: string)}
  <button class="go" onclick={() => nav.go(tab, anchor)} title="Ouvrir : {label}">→ {label}</button>
{/snippet}

{#snippet example(said: string, pasted: string, note?: string)}
  <div class="example">
    <div class="line"><span class="who">Tu dis</span><i>« {said} »</i></div>
    <div class="line"><span class="who strong">Scriibe colle</span><b>{pasted}</b></div>
    {#if note}<div class="note">{note}</div>{/if}
  </div>
{/snippet}

{#snippet tip(text: string)}
  <div class="tip">💡 {text}</div>
{/snippet}

{#snippet question(q: string, answer: string)}
  <div class="q"><b>{q}</b><div>{answer}</div></div>
{/snippet}

<h1>Aide</h1>
<p class="muted lead">Comment se servir de Scriibe, et à quoi sert chaque chose. Les boutons <b>→</b> ouvrent le bon réglage ; le <b>?</b> de chaque carte ouvre l’explication qui la concerne.</p>

<div class="layout">
  <ul class="toc">
    {#each helpSections as sec (sec.id)}
      <li><button class:active={open === sec.id} onclick={() => pick(sec.id)}>{sec.title}</button></li>
    {/each}
  </ul>

  <section class="card body">
    {#if open === 'start'}
      <h2>Premiers pas</h2>
      <p>Scriibe écrit à ta place dans n’importe quelle appli : tu maintiens une touche, tu parles, tu relâches, et le texte, propre et ponctué, se colle là où est ton curseur.</p>
      <ol class="steps">
        <li><b>Installe ce qu’il faut.</b> Pour tout faire sur cet ordinateur, il faut <b>whisper-server</b> et un modèle Whisper (la transcription), puis, si tu veux une mise au propre sans Internet, <b>llama-server</b> et un modèle. Les cartes d’installation (Réglages → Moteurs) disent ce qui manque et, avant tout téléchargement, montre le nom, la taille réelle et la source : rien ne se télécharge sans ton accord.{#if mac} Sur Mac, les deux programmes s’installent avec Homebrew (« brew install whisper-cpp » puis « brew install llama.cpp », dans le Terminal) ; les cartes proposent ensuite les modèles.{/if}</li>
        <li><b>Choisis tes moteurs.</b> Whisper et llama.cpp font tout sur cet ordinateur, sans Internet ; Deepgram et OpenRouter passent par des services en ligne et demandent une clé : ils servent de dépannage quand l’ordinateur est trop lent. Au premier lancement, l’assistant de premier lancement (il s’ouvre sur le Tableau de bord) mesure ton ordinateur et propose la chaîne qui lui convient.</li>
        <li><b>Dicte.</b> Clique dans un document, maintiens <b>{dictate}</b>, dis une phrase, relâche : elle s’écrit.</li>
      </ol>
      <div class="links">
        {@render go('Installer les programmes et modèles', 'install')}
        {@render go('Les moteurs', 'settings', 'moteurs')}
        {@render go('La touche et le micro', 'settings', 'dictee')}
        {@render go('Tableau de bord', 'dashboard')}
      </div>
      <p>La fenêtre a cinq pages : <b>Tableau de bord</b>, <b>Historique</b>, <b>Dictionnaire</b>, <b>Réglages</b> et <b>Aide</b>. Les Réglages ont des onglets : Dictée, Moteurs, Apparence, Modes, Mesures et essais, et Mise à jour. Chaque carte se replie d’un clic sur son titre (le pli est retenu) et son bouton « ? » ouvre la section de l’Aide qui la concerne. Réglages → Apparence : thème clair, sombre ou automatique (celui du système), et langue de l’interface. Le Tableau de bord te salue par ton prénom : par défaut celui du compte de l’ordinateur (sous Windows, le nom de connexion), modifiable dans Réglages → Dictée, « Prénom dans la salutation » (laisse vide pour reprendre celui du compte).</p>
      {@render tip('Pour vérifier le micro et voir ce que Whisper entend, lis une phrase du banc de dictée (Réglages → Mesures et essais) : elle montre le niveau du micro avant d’aller dicter dans une vraie appli.')}
      <div class="links"><button class="go" onclick={() => nav.openAssistant()} title="Ouvrir l’assistant sur le Tableau de bord">→ Relancer l’assistant de premier lancement</button></div>

      <h3>Ce qui se passe entre ta voix et le texte</h3>
      <p>Quand tu relâches la touche, ta phrase passe par plusieurs étapes, chacune faite par quelqu’un (ou quelque chose) de différent :</p>
      <ol class="steps">
        <li><b>Le micro</b> enregistre ce que tu dis pendant que tu maintiens la touche. Rien d’autre : c’est un simple fichier de son.</li>
        <li><b>La transcription</b> (Whisper, ou Deepgram en ligne) écoute ce son et écrit les mots. C’est comme un sous-titreur : il entend, il n’interprète pas. Il écrit ce qu’il croit entendre, avec une ponctuation approximative, et il peut se tromper sur un nom propre ou un mot rare.</li>
        <li><b>Les corrections de texte</b> appliquent des règles fixes, sans modèle et sans attente : les signes dictés (« point à la ligne », « ouvrez la parenthèse »), les listes, et ton dictionnaire (« cloud code » → « Claude Code »).</li>
        <li><b>La mise au propre</b> est faite par un <b>modèle de langage</b> (llama.cpp sur cet ordinateur, ou OpenRouter en ligne). Lui comprend le sens : il retire les « euh » et les reprises, corrige la ponctuation, reformule si le mode le demande, et traduit si tu tiens la touche de traduction. Il suit la consigne du mode choisi, et un garde-fou vérifie qu’il n’a pas répondu à ta place ou inventé du texte.</li>
        <li><b>La typographie française</b> met les espaces avant « ? » ou « : », les « » et les apostrophes comme il faut, puis Scriibe <b>colle</b> le résultat là où est ton curseur.</li>
      </ol>
      {@render tip('Deux modèles différents travaillent donc l’un après l’autre : celui qui écoute (Whisper) et celui qui rédige (le modèle de mise au propre). Ils ne servent pas à la même chose, et on peut choisir l’un en local et l’autre en ligne.')}

      <h3>Un programme, un modèle : à quoi sert le choix ?</h3>
      <p>Chaque étape a deux morceaux. Le <b>programme</b> (<i>whisper-server</i>, <i>llama-server</i>) est le moteur : petit, il sait faire tourner un modèle. Le <b>modèle</b> est un gros fichier qui contient tout ce que l’intelligence artificielle a appris : c’est lui qui fait la qualité. Un même programme peut faire tourner des modèles très différents, d’où le choix.</p>
      <p>Choisir un modèle, c’est choisir un compromis entre quatre choses :</p>
      <ul>
        <li><b>La qualité.</b> Un gros modèle entend mieux (Whisper « small » plus que « base ») ou rédige mieux (un modèle de 3 milliards de paramètres plus qu’un de 0,5 milliard). Un petit modèle se trompe plus : un mot mal entendu, une phrase mal reformulée, une reprise « non pardon » non corrigée.</li>
        <li><b>La vitesse.</b> Un gros modèle fait plus de calculs, donc il met plus longtemps. Sur un portable sans vraie carte graphique, Whisper « small » prend plusieurs fois plus de temps que « base », et un modèle de rédaction de 3 milliards de paramètres met une dizaine de secondes par phrase. Un service en ligne répond en deux secondes environ, parce qu’il tourne sur des machines puissantes.</li>
        <li><b>La place et la mémoire.</b> Un modèle prend de quelques centaines de Mo à plusieurs Go sur le disque, et autant en mémoire quand il travaille. Une carte graphique de 2 Go ne tient pas un modèle de 2 Go : Scriibe bascule alors tout seul sur le processeur.</li>
        <li><b>La confidentialité et le coût.</b> En local, ta voix et ton texte restent sur l’ordinateur, et c’est gratuit. En ligne (Deepgram, OpenRouter), c’est rapide, mais ta voix ou ton texte part chez le service, et il faut une clé (souvent payante).</li>
      </ul>
      <p>Sans idée précise, laisse l’assistant choisir : il mesure ton ordinateur et propose la combinaison qui convient, avec les temps qu’on a vraiment mesurés. Tu peux le <b>relancer quand tu veux</b> (bouton ci-dessus, ou Réglages → Moteurs) si tu changes d’ordinateur, de carte graphique ou d’avis.</p>
      <div class="links">
        {@render go('Choisir les moteurs', 'settings', 'moteurs')}
        {@render go('Installer un modèle', 'install')}
        {@render go('Comparer avec le banc de dictée', 'bench')}
      </div>

    {:else if open === 'dictate'}
      <h2>Dicter et traduire</h2>
      <p>Maintiens <b>{dictate}</b> tout le temps où tu parles ; au relâchement, Scriibe transcrit, met au propre et colle. Parle naturellement : les « euh », les reprises et les répétitions sont retirés pour toi (sauf en mode Littéral, ou si le moteur de mise au propre est « Aucune »).</p>
      {@render example('euh, le rendez-vous est à dix heures, non pardon, à onze heures', 'Le rendez-vous est à 11 h.', 'Une reprise (« non pardon », « enfin ») ne garde que la version corrigée.')}
      {@render example('est-ce que tu peux m’envoyer le devis avant vendredi midi', 'Est-ce que tu peux m’envoyer le devis avant vendredi midi ?', 'Une question ou une demande reste une question : Scriibe l’écrit, il n’y répond pas (voir « Problèmes fréquents » si cela arrive).')}
      <h3>Traduire en dictant</h3>
      <p>Ajoute <b>{translate}</b> pendant la dictée : tu parles en français, le texte est collé en <b>{language}</b>. La langue se choisit dans Réglages → Dictée (« Traduire vers »), la touche aussi (« Touche tenue en plus pour traduire »).</p>
      {@render example('tu peux me rappeler demain matin', 'Can you call me back tomorrow morning?', 'Ici en anglais.')}
      <p>La traduction passe toujours par le moteur de mise au propre, même en Littéral : avec OpenRouter, le texte part alors en ligne. Sans moteur (« Aucune »), {translate} ne traduit pas : le texte reste en français. <i>Pourquoi :</i> c’est le modèle qui traduit, en un seul appel avec la mise au propre ; sans modèle, rien ne peut le faire.</p>
      {@render tip('Langue de l’interface et langue dictée sont deux réglages différents. La première change les textes de Scriibe ; la seconde (Réglages → Dictée → « Langue dictée ») dit en quelle langue tu parles : c’est celle que Whisper écoute, et celle des phrases du Banc d’essai et de l’essai d’un mode. Si tu dictes en français avec une interface en anglais, laisse « Français » ; pour essayer le banc en anglais, choisis « Anglais », puis remets « Français » ensuite, sinon Whisper transcrira ta voix en anglais.')}
      <h3>Choisir ses touches</h3>
      <p>Par défaut : <b>{defaultDictate}</b> pour dicter, <b>{defaultTranslate}</b> en plus pour traduire. Pour changer : Réglages → Dictée → « Changer… », puis <b>appuie directement</b> sur la touche ou la combinaison voulue (par exemple {mac ? 'Option gauche + F9' : 'Ctrl gauche + F9, ou la touche Windows'}) et relâche tout : Scriibe la retient, et « Tester ma touche » vérifie qu’elle est bien reconnue.</p>
      <ul>
        <li>{#if mac}Possibles : Option, Commande (⌘), Fn, Ctrl, Maj, les touches de fonction (F1 à F20), Début, Fin, Page préc. et Page suiv. Pas possibles : les lettres, les chiffres, Espace, Entrée et les flèches. Sur la plupart des Mac, F1 à F12 demandent Fn en plus, sauf si les « touches de fonction standard » sont activées.{:else}Possibles : Ctrl, Alt gauche, Maj, la touche Windows, les touches de fonction, « &lt; &gt; » (à gauche de W), Pause, Inser… Pas possibles : les lettres, les chiffres, Espace, Entrée, et AltGr (il sert à taper « @ » ou « € » sur un clavier AZERTY).{/if}</li>
        <li>{#if mac}<b>Des touches de modification seules</b> : Ctrl et Maj demandent de les tenir une fraction de seconde (0,3 s) avant que l’écoute commence ; Option, ⌘ et Fn écoutent tout de suite. <i>Pourquoi :</i> les raccourcis clavier (⌘C, Maj+lettre) appuient aussi sur Ctrl et Maj, et ne doivent pas ouvrir le micro.{:else}<b>Des touches de modification seules</b> (Maj gauche, Windows, Ctrl + Alt…) : il faut les tenir une fraction de seconde (0,3 s) avant que l’écoute commence. <i>Pourquoi :</i> les raccourcis clavier (Ctrl+C, Maj+lettre, Windows+E) les appuient aussi, et ne doivent pas ouvrir le micro.{/if}</li>
        {#if !mac}<li><b>Alt gauche et Windows</b> ouvriraient d’ordinaire le menu de l’appli ou le menu Démarrer au relâchement, et le collage perdrait le focus : Scriibe glisse une touche sans effet pendant l’appui pour l’éviter.</li>{/if}
        <li>Une touche ordinaire de la combinaison (F9, « &lt; &gt; ») n’écrit rien dans l’appli pendant la dictée.</li>
        <li>{#if mac}<b>La touche Fn</b> est proposée sur Mac, mais n’a pas encore été essayée comme touche de dictée : si « Changer… » ne reçoit rien en l’appuyant, choisis une autre touche.{:else}<b>La touche Fn</b> n’est pas vue par {linux ? 'Linux' : 'Windows'} sur la plupart des portables : le clavier la gère lui-même. Si « Changer… » ne reçoit rien en l’appuyant, c’est le cas ; choisis une autre touche.{/if}</li>
      </ul>
      <h3>Bon à savoir</h3>
      <ul>
        <li>Une <b>autre touche</b> pendant l’appui (par exemple {dictate} + une lettre) annule la dictée : c’est un raccourci, pas une dictée. <i>Pourquoi :</i> {copyKey} ne doit pas ouvrir le micro pour rien.</li>
        <li>Une dictée de <b>trois mots ou moins</b> est collée sans passer par le modèle, en mode Simple (hors traduction) : c’est plus rapide, mais un « euh » y reste.</li>
        <li>Ce que tu avais copié <b>revient dans le presse-papiers</b> après le collage (réglable). <i>Pourquoi :</i> Scriibe passe par le presse-papiers pour coller, et ne doit pas te le prendre. Seul le texte est remis : une image ou des fichiers copiés avant la dictée sont perdus.</li>
        <li><b>« Mettre en pause la musique ou la vidéo »</b> (Réglages → Dictée, coupé au départ) : une vidéo ou une musique qui joue s’arrête quand tu maintiens la touche et reprend quand tu relâches. Scriibe regarde quelles applis sortent du son. Un lecteur qui a sa fenêtre (VLC, Spotify…) reçoit la commande Lecture/Pause directement ; les navigateurs la reçoivent par la touche Lecture/Pause du clavier. {#if mac}<b>Avec Chrome</b>, Chrome garde le son « ouvert » une dizaine de secondes après la pause d’une vidéo : pour ne pas relancer une vidéo déjà arrêtée, Scriibe demande à Chrome si une page lit vraiment quelque chose, puis arrête et relance lui-même la vidéo par la page (la touche du clavier n’arrête pas toujours YouTube). Pour cela, une fois : dans Chrome, menu Présentation → Développeur → « Autoriser JavaScript provenant d’Apple Events », puis accepter la demande de macOS « Scriibe veut contrôler Google Chrome ». Sans cela, une vidéo mise en pause depuis moins de dix secondes peut repartir pendant la dictée. {/if}Si cette touche lance une autre appli que celle qui jouait, Scriibe l’annule. Pendant un appel (Teams, Meet, Zoom…), Scriibe ne touche à rien, et si tu lances autre chose pendant la dictée, il ne relance pas l’ancien média. <i>Pourquoi :</i> interrompre un appel ou relancer le mauvais lecteur serait pire que de laisser jouer.</li>
        <li>{#if linux}Sous Linux, il n’y a pas encore d’icône : la page de Scriibe s’ouvre dans le navigateur, et « Quitter Scriibe » (dans la page) arrête le programme.{:else}Le <b>menu de l’icône</b>, {trayNear}, ouvre la fenêtre, le dossier des données et le journal, ou quitte Scriibe.{/if}</li>
        <li><b>Une sélection</b> est remplacée par la dictée : si un mot ou un groupe de mots est sélectionné quand tu relâches la touche, le texte dicté prend sa place <b>sans majuscule au début ni point, virgule ou point-virgule à la fin</b>, puisqu’il s’insère au milieu d’une phrase (le « ? », le « ! » et les points de suspension restent ; les noms de ton dictionnaire et les sigles gardent leur majuscule). <i>Pourquoi :</i> Whisper met une majuscule et un point à chaque dictée, ce qui ne va pas au milieu d’une phrase. {#if mac}Scriibe envoie un ⌘C pour savoir si un texte est sélectionné et remet aussitôt ton presse-papiers.{:else}Cela ne marche pour le moment que sur Mac.{/if}</li>
      </ul>
      <div class="links">{@render go('Réglages de la dictée', 'settings', 'dictee')}</div>

    {:else if open === 'modes'}
      <h2>Les modes</h2>
      <p>Le mode décide de ce que devient ton texte. Tous gardent tes informations et tes questions, et ne doivent jamais répondre à ce que tu dictes (un garde-fou y veille) ; les modes qui reformulent changent le ton, et Courrier vouvoie le destinataire.</p>
      {@render example('bon ben j’ai regardé le devis que tu m’as envoyé et franchement ça me va sauf le prix de la livraison', 'J’ai regardé le devis que tu m’as envoyé et franchement ça me va, sauf le prix de la livraison.', 'Simple : retire les « euh », ponctue, garde tes mots.')}
      {@render example('bon ben j’ai regardé le devis que tu m’as envoyé et franchement ça me va sauf le prix de la livraison', 'J’ai examiné le devis que tu m’as adressé et, franchement, cela me convient, à l’exception du coût de la livraison.', 'Soutenu : reformule dans un français soigné.')}
      {@render example('est-ce que tu as pensé à réserver le restaurant pour l’anniversaire de maman', 'Tu as pensé à réserver le resto pour l’anniv de maman ?', 'Un mode personnel « SMS », avec trois exemples.')}
      <ul>
        <li><b>Littéral</b> : aucun modèle, seulement les signes dictés, le dictionnaire et la typographie.</li>
        <li><b>Simple</b> (par défaut), <b>Soutenu</b>, et quatre modes professionnels : sophrologie, informatique et IA, courrier administratif, commercial.</li>
        <li><b>Apaisé (CNV)</b> : pour les échanges tendus (WhatsApp, réseaux sociaux). Il réécrit en communication non violente : les faits sans jugement, ton ressenti et ton besoin à la première personne, puis une demande claire posée comme une question. Il réécrit librement : relis le texte avant d’envoyer, et vérifie qu’il n’invente rien que tu n’as pas dit.</li>
      </ul>
      <h3>La fidélité à ta dictée</h3>
      <p>Après la réponse du modèle, un garde-fou vérifie qu’il est resté fidèle à tes mots ; sinon, ton texte brut est collé et le message « Reformulation écartée » s’affiche. Chaque mode a un cran de fidélité, et tu peux le choisir pour Simple (Réglages → Modes) et pour tes modes personnels :</p>
      <ul>
        <li><b>Littéral</b> : au moins 90 % de tes mots gardés. Seules la ponctuation et de petites corrections passent.</li>
        <li><b>Léger</b> (Simple par défaut) : au moins 75 % de tes mots gardés sur une dictée de huit mots ou plus. Les mots de commande dictés (« parenthèse », « deux points », « retour à la ligne ») ne comptent pas.</li>
        <li><b>Reformulé</b> : le texte doit rester proche de ta dictée, mais les mots peuvent changer (modes Soutenu et professionnels). Pour Simple, ce cran desserre seulement le contrôle : la consigne reste « garde tes mots ».</li>
        <li><b>Libre</b> : la part de mots gardés n’est plus contrôlée (mode Apaisé). La longueur, la question qui reste une question, les signes dictés et l’absence de « Oui, » ou « Voici » en tête restent contrôlés.</li>
      </ul>
      <p>Pour que les modes qui réécrivent accordent ce qui te concerne (« je me sens délaissé » ou « délaissée »), indique ton genre dans Réglages → Dictée : « Masculin » ou « Féminin ». Par défaut (« Non précisé »), rien n’est demandé au modèle, qui devine. Simple et Littéral gardent tes mots et n’en tiennent pas compte.</p>
      <h3>Créer ton mode</h3>
      <ol class="steps">
        <li>Réglages → Modes → « Mes modes » : un nom, une consigne (« Écris comme un SMS court et amical »).</li>
        <li>Choisis la fidélité à ta dictée : Littéral et Léger gardent tes mots avec la consigne en plus ; Reformulé et Libre réécrivent dans le style, comme les modes métiers.</li>
        <li>Ajoute deux ou trois <b>exemples</b> : une dictée telle que tu la dirais, et le texte que tu attends. Ce sont eux qui donnent vraiment le style.</li>
        <li>Choisis-le dans la liste, puis dicte un texte pour vérifier que les questions restent des questions et que les reprises sont corrigées.</li>
      </ol>
      {@render tip('Avec un petit modèle sur un ordinateur ordinaire, une consigne seule change peu le texte : mesuré sur Mac, un mode « SMS » sans exemples écrivait comme Simple, et avec trois exemples il écrivait « resto » et « anniv ». Les exemples comptent plus que la consigne.')}
      <div class="links">{@render go('Les modes', 'modes')}</div>

    {:else if open === 'dictionary'}
      <h2>Dictionnaire et signes</h2>
      <p>Le dictionnaire apprend à Scriibe tes mots à toi, et les signes se dictent à voix haute.</p>
      <h3>Signes dictés</h3>
      {@render example('le colis ouvrez la parenthèse le gros fermez la parenthèse est arrivé point à la ligne merci encore', 'Le colis (le gros) est arrivé.\nMerci encore.', 'Les variantes sont reconnues : « ouvre » ou « ouvrez », « ferme » ou « fermez », « retour à la ligne », « nouveau paragraphe »…')}
      <p>« À la ligne » dit seul coupe aussi la ligne (« merci beaucoup, à la ligne, cordialement »), à condition d’être isolé par de la ponctuation : au début de la dictée, ou entre virgules ou points, ce que Whisper écrit quand tu marques une pause. Dans une phrase, il reste du texte : « aller à la ligne 5 », « arrivé à la ligne d’arrivée ». Devant un début de liste (« ma liste à la ligne tiret du lait »), « à la ligne » compte aussi sans ponctuation. Ailleurs, « retour à la ligne » est plus sûr : il marche même sans pause. <i>Pourquoi :</i> « à la ligne » est aussi une expression ordinaire ; sans ces règles, Scriibe couperait tes phrases au mauvais endroit.</p>
      {@render tip('Les signes dépendent de ce que Whisper entend. Un modèle plus petit ou un micro faible entend parfois « ouvrait » à la place de « ouvrez » : la conversion ne se déclenche alors pas. Dis les signes distinctement, ou ajoute une correction automatique pour la variante entendue.')}
      <h3>Listes</h3>
      {@render example('les courses retour à la ligne tiret du lait retour à la ligne tiret du pain', 'Les courses\n- du lait\n- du pain', '« tiret » ou « puce » en début de ligne donne un tiret. Pour une liste numérotée : « point 1 », « point 2 »… (en lettres, « point un, » avec une pause) ou « numéro suivant » pour passer au numéro d’après.')}
      <p>Ces mots ne comptent qu’au début d’une ligne, c’est-à-dire au début de la dictée ou après un saut de ligne (« retour à la ligne », « point à la ligne », « nouveau paragraphe ») : « il a mis un tiret entre les deux mots » reste une phrase ordinaire. Mais une ligne qui commence par « Point 3 du contrat » devient « 3. du contrat » : c’est le prix de la dictée à la voix. « Tirer », « tirez », « tiré » ou « tirée », que Whisper entend parfois à la place de « tiret », comptent aussi, mais seulement après un saut de ligne. Pas en dictée anglaise ni après une traduction.</p>
      <h3>Mes mots</h3>
      <p>Noms, marques, jargon (« Claude Code », le nom d’un client) : ajoutés ici, ils sont donnés comme <b>indices</b> à la transcription et au modèle, et donc mieux reconnus. Pour une écriture certaine, ajoute aussi une correction automatique : un indice aide, il ne garantit rien.</p>
      <h3>Corrections automatiques</h3>
      <p>Un mot entendu est remplacé par celui que tu choisis : utile pour un mot toujours mal entendu (« cloud code » → « Claude Code »), ou pour un signe à toi (« trois points d’exclamation » → !!!). La correction s’applique toujours, à chaque dictée.</p>
      <h3>Adresses e-mail : règle ton dictionnaire</h3>
      <p>Whisper entend mal les adresses : il efface souvent « arobase » et « gmail », ou écrit ton nom autrement (« jean point dupont point com »). Les signes ne peuvent pas le deviner. Dicte en détachant bien chaque mot (« jean arobase gmail point com »), et pour <b>ta propre adresse</b>, ajoute une correction : « Entendu » ce que Whisper écrit (relis-le dans l’historique), « Écrit » ton adresse complète. Ajoute de la même façon plusieurs « Entendu » pour « @ » (« arobase », « aro base », « a rond »…) et, si Whisper écrit ton nom autrement, une correction pour ton nom : le dictionnaire se règle au fil de l’eau, avec tes propres erreurs d’écoute.</p>
      <h3>Commandes vocales</h3>
      <p>Quand du texte est sélectionné, une phrase dite seule (rien d’autre dans la dictée) agit sur lui au lieu d’être collée. Sans sélection, ou avec d’autres mots autour, la phrase reste du texte ordinaire. Ni le texte sélectionné ni la commande ne sont gardés dans l’historique.</p>
      <ul>
        <li><b>Ajouter au dictionnaire</b> : « ajouter au dictionnaire » ouvre cet onglet avec le texte sélectionné dans « Entendu ».</li>
        <li><b>Caractères autour</b> : « entre parenthèses », « entre guillemets » (« » en français, " " en anglais), « entre crochets » ou « entre accolades » entourent la sélection ; un seul ⌘Z l’annule (sur Mac, dans Chrome et ses dérivés, où le texte est tapé, il en faut plusieurs).</li>
        <li><b>Gras, italique, souligné</b> : « mettre en gras », « mettre en italique », « mettre en souligné » appuient le raccourci de l’appli (⌘B, ⌘I, ⌘U sur Mac, modifiables). Scriibe n’écrit pas de Markdown : il se sert de l’appli.</li>
        <li><b>Mes commandes</b> : dans la carte « Commandes vocales » (onglet Dictionnaire), donne un titre, une phrase et un raccourci de l’appli (par exemple ⌘⇧X) : dire la phrase appuie ce raccourci, avec ou sans texte sélectionné selon ton choix.</li>
      </ul>
      {@render tip('Les phrases se changent dans la carte « Commandes vocales » de l’onglet Dictionnaire : deux mots au moins, et une phrase ne peut pas servir à deux commandes. Un interrupteur coupe toutes les commandes. Elles ne marchent pour l’instant que sur Mac.')}
      <div class="links">{@render go('Ouvrir le dictionnaire', 'dictionary')}</div>

    {:else if open === 'indicator'}
      <h2>L’indicateur</h2>
      <p>Pendant la dictée, une petite pastille en bas de l’écran montre où en est Scriibe, sans que tu aies à regarder ailleurs. Elle ne prend jamais le focus : ce que tu tapes continue d’aller dans ton appli. <i>Pourquoi :</i> sans cela, tu ne saurais pas si Scriibe t’entend, avant d’avoir parlé dans le vide.</p>
      <div class="samples">
        {#each [['Écoute', 'listening'], ['Mise au propre', 'working'], ['Collé', 'done']] as [name, state] (state)}
          <div class="sample">
            <div class="stagebox"><OverlayView view={sample(state as View['state'])} levels={curve} inline /></div>
            <span class="muted">{name}</span>
          </div>
        {/each}
      </div>
      <ul>
        <li><b>Écoute</b> : « J’écoute… », ou un point ou une courbe qui suit ta voix selon le style. « Je n’entends rien… » ou « Je t’entends à peine… » si le micro est trop faible.</li>
        <li><b>Mise au propre</b> : un arc qui tourne (avec « Mise au propre… » dans la pastille), le temps de transcrire et de mettre au propre. Pour une traduction, la langue est annoncée.</li>
        <li><b>Collé</b> : un ✓ confirme que le texte est arrivé.</li>
        <li>{#if linux}Sous Linux, il n’y a pas encore d’icône : l’état se lit dans la barre latérale de la page.{:else}<b>{trayIcon}</b> change toujours de couleur : <b>gris</b> au repos, <b>rouge</b> pendant l’écoute, <b>orange</b> pendant le traitement, <b>vert</b> une fois le texte collé, <b>bleu</b> pour un message. Elle sert quand la pastille est coupée.{/if}</li>
        <li>Trois styles (pastille, point sobre, vumètre), trois positions, cinq couleurs et une transparence réglable. La pastille est active par défaut ; elle se coupe dans les réglages. L’apparence de la fenêtre (claire, sombre ou celle du système) se règle au même endroit, dans Réglages → Apparence.</li>
      </ul>
      <div class="links">{@render go('Régler l’indicateur', 'settings', 'indicateur')}</div>

    {:else if open === 'measures'}
      <h2>Mesurer et comparer</h2>
      <p>Ces outils ne servent pas au quotidien : ils répondent à une question précise. Choisis-les selon ce que tu veux savoir.</p>
      {@render question('Scriibe marche-t-il bien chez moi, en vrai ?', 'Le Banc d’essai (Réglages → Mesures et essais) : tu lis 17 phrases pièges (hésitations, reprises, questions, signes dictés), et chacune est contrôlée automatiquement. Il montre ce que Whisper a entendu, ce qui serait collé, les temps de chaque étape, et le niveau du micro.')}
      {@render question('Les phrases du banc ne sont pas dans la bonne langue.', 'Elles suivent la langue dictée (Réglages → Dictée → « Langue dictée »), pas la langue de l’interface : Whisper transcrit dans la langue choisie, donc on lit des phrases de cette langue. Pour lire les 17 phrases en anglais, choisis « Anglais », puis remets « Français » quand tu as fini. Le test de frappe, lui, suit la langue de l’interface.')}
      {@render question('Qu’est-ce qu’un changement a modifié ?', 'Chaque passage du banc est gardé avec ses réglages (version, moteurs, modèles, micro). Refais un passage après avoir changé un réglage ou installé une mise à jour, puis « Comparer deux passages » : les mots changés, et les contrôles gagnés ou perdus. C’est ainsi qu’on juge Whisper « small » contre « base », ou le cloud contre le local.')}
      {@render question('Quel moteur choisir sur mon ordinateur ?', 'L’assistant du Tableau de bord propose une chaîne selon les cœurs, la mémoire et la carte graphique, avec les temps mesurés sur un portable ordinaire. Un passage du banc dit ensuite si elle te convient. Attention : un seul passage de 17 phrases donne une tendance, pas une mesure ferme ; relire les phrases change aussi ce que Whisper entend.')}
      {@render question('Combien de temps j’attends ?', 'L’Historique donne, pour chaque dictée, le temps de transcription, de mise au propre et le total depuis le relâchement de la touche. Le Journal garde une ligne par étape (jamais le texte).')}
      {@render question('Comment est calculé le temps gagné du Tableau de bord ?', `Pour chaque dictée, Scriibe compare le temps passé à parler au temps qu’il t’aurait fallu pour taper le même texte, à ta vitesse au clavier (réglée à ${v?.typingWPM ?? 40} mots par minute dans « Ma vitesse au clavier », pour tout l’historique, jours purgés compris : il n’en reste que les totaux du jour).`)}
      {@render question('Quelle est ma vraie vitesse de frappe ?', 'Dans le Tableau de bord, « Ma vitesse au clavier » : tu recopies un texte de quelques lignes, le chrono part à ta première lettre, les fautes passent en rouge, et l’essai s’arrête quand le texte est identique (ou avec « Terminer »). Le résultat est net (mots du texte × part de lettres justes ÷ minutes). Ne sont pas retenus : un essai sous 90 % de lettres justes, plus court que 12 s, plus long que 10 min ou au-delà de 200 mots/min. « Utiliser » reporte la moyenne des essais retenus dans ta vitesse au clavier ; le temps gagné est alors recalculé pour tout l’historique.')}
      {@render question('Le rapport, à quoi sert-il ?', 'Réglages → Mesures et essais → « Rapport à me coller » → « Copier le rapport » met dans le presse-papiers l’état de l’installation, le dernier passage du banc et les dernières lignes du journal : c’est ce qu’il faut coller pour qu’on t’aide à comprendre un problème.')}
      <div class="links">{@render go('Mesures et essais', 'bench')}{@render go('Rapport et journal', 'journal')}</div>

    {:else if open === 'privacy'}
      <h2>Sur cet ordinateur ou en ligne</h2>
      <p>{privacySummary}</p>
      <ul>
        <li><b>Whisper</b> (transcription) et <b>llama.cpp</b> (mise au propre) tournent sur cet ordinateur : ta voix et ton texte restent sur l’ordinateur.</li>
        <li><b>Deepgram</b> reçoit l’enregistrement de chaque dictée, une fois la touche relâchée (rien en dehors de tes dictées) ; <b>OpenRouter</b> reçoit le texte transcrit pour le mettre au propre.</li>
        <li><b>Mémoire</b> : Whisper s’arrête après 10 minutes sans dictée et revient à l’appui de la touche, pendant que tu parles ; le modèle de mise au propre de llama.cpp se libère après 30 minutes par défaut (réglable : 2 h, 8 h ou « tant que Scriibe tourne »). <i>Pourquoi :</i> ils occupent plusieurs Go de mémoire. Après une libération, la première dictée est plus lente : sur un ordinateur sans carte graphique, comptez 15 à 45 s pour le modèle local ; le préchargement au lancement l’évite pour la première.</li>
        <li>Les mots de « Mes mots » accompagnent la dictée : vers Deepgram et OpenRouter si tu les utilises.</li>
        <li>La traduction passe toujours par le moteur de mise au propre, même en Littéral.</li>
        <li>Choisis « Aucune » comme mise au propre pour ne charger aucun modèle : le texte est transcrit tel quel (signes dictés, dictionnaire, typographie). Les modes et la traduction sont alors indisponibles.</li>
        <li>L’historique, le dictionnaire, les réglages et les passages du banc restent sur cet ordinateur, dans le dossier de données (Réglages → Emplacements).</li>
        <li>Historique, carte « Confidentialité de l’historique » : décoche « Garder le texte des dictées » pour ne garder que les statistiques (date, appli, mots, durée, temps), ou « Ne garder que les derniers jours » : les dictées plus anciennes sont supprimées, il n’en reste que des totaux par jour, sans texte, pour le Tableau de bord. Un aperçu du nombre de dictées touchées s’affiche avant toute suppression. Cela ne change pas ce que reçoivent Deepgram ou OpenRouter pendant la dictée.</li>
        <li>Les clés de Deepgram et d’OpenRouter ne sont jamais affichées ni écrites dans le journal ; {#if mac}elles sont rangées dans le trousseau du Mac.{:else if linux}sous Linux, Scriibe ne les range pas : tu les donnes par des variables d’environnement, avant de lancer Scriibe.{:else}elles sont rangées dans le Gestionnaire d’identifiants de Windows.{/if}</li>
      </ul>
      <div class="links">{@render go('Les moteurs', 'settings', 'moteurs')}{@render go('Confidentialité de l’historique', 'history', 'conservation')}</div>

    {:else}
      <h2>Problèmes fréquents</h2>
      {@render question('Scriibe n’entend rien.', `Le niveau du micro se voit dans le banc d’essai (Réglages → Mesures et essais). ${mac ? 'Sur Mac, vérifie « Microphone » dans Réglages Système → Confidentialité et sécurité : l’interrupteur de Scriibe doit être activé ; quitte ensuite Scriibe et relance-le.' : linux ? 'Vérifie le volume d’entrée dans les réglages son du système.' : 'Sous Windows, vérifie Paramètres → Confidentialité → Microphone : l’accès au micro doit être autorisé.'} Vérifie aussi le micro choisi (Réglages → Dictée → Micro).`)}
      {@render question('Rien ne se colle, ou une phrase manque.', `Regarde le journal (Réglages → Mesures et essais, en bas de l’onglet) : chaque dictée y laisse une ligne « collage: … signes envoyés à … ». Si la ligne dit « copié seulement », le texte est dans le presse-papiers : ${pasteKey} le colle. ${pasteNote} Si la ligne dit que le texte a été envoyé mais que rien n’apparaît, l’appli n’a pas lu le presse-papiers : essaie dans ${editor} pour voir si le problème vient de l’appli, et copie le rapport.`)}
      {@render question('Ma touche ne fait rien.', `Réglages → Dictée → « Tester ma touche », puis maintiens la touche : si Scriibe ne passe pas en écoute, un autre programme la capte peut-être (outil de macros, logiciel du clavier). Choisis une autre touche ou une combinaison. ${mac ? 'Sur Mac, vérifie surtout que Scriibe a l’autorisation « Surveillance de l’entrée » (Réglages Système → Confidentialité et sécurité), puis quitte Scriibe et relance-le : sans elle, le journal dit que macOS refuse la touche globale.' : linux ? '' : 'Si le journal parle d’un crochet clavier refusé, relance Scriibe.'}`)}
      {@render question('La pastille reste bloquée en attente alors que j’ai relâché la touche.', 'Le plus souvent, il suffit d’appuyer de nouveau sur la touche de dictée, puis de la relâcher : la pastille se débloque. Si elle reste bloquée malgré cela, ouvre le journal (Réglages → Mesures et essais, en bas de l’onglet) pour voir la dernière étape notée, ou quitte Scriibe et relance-le.')}
      {@render question('Le début de ma phrase est perdu.', `Avec un casque Bluetooth, le système bascule en mode « mains libres » à l’ouverture du micro (1 à 3 s) : attends que la pastille affiche « J’écoute… » avant de parler, ou mieux, choisis un micro filaire ou USB dans Réglages → Dictée → Micro. Un micro de portable est souvent faible : le Banc d’essai montre son niveau.`)}
      {@render question('La première phrase après une longue pause est lente.', 'Pour rendre de la mémoire, Whisper s’arrête après 10 minutes sans dictée et le modèle de mise au propre de llama.cpp après la durée réglée (« Garder le modèle chargé »). Ils reviennent à l’appui de la touche : Whisper en une seconde environ, le modèle local en 15 à 45 s sans carte graphique. Choisis « tant que Scriibe tourne » pour le garder chargé.')}
      {@render question('Il manque un programme ou un modèle.', `Réglages → Moteurs (cartes d’installation) : whisper-server, un modèle Whisper (ggml-base.bin pour commencer), et pour une mise au propre locale llama-server et un modèle GGUF. Chaque téléchargement montre sa taille et sa source avant de demander ton accord ; une connexion coupée reprend là où elle s’était arrêtée.${mac ? ' Sur Mac, les programmes viennent de Homebrew : « brew install whisper-cpp » et « brew install llama.cpp ».' : ''}`)}
      {@render question('Avec llama.cpp, la mise au propre ne démarre pas.', 'Il faut llama-server et un fichier modèle .gguf (Réglages → Moteurs, cartes d’installation, qui disent ce qui manque). Si le journal du moteur parle de mémoire, essaie « Processeur seulement » dans Réglages → Moteurs → Carte graphique : sur une carte de 2 Go, le modèle ne tient pas toujours.')}
      {@render question('Ma question a reçu une réponse au lieu d’être écrite.', 'Le garde-fou écarte la plupart de ces sorties : un mode qui reformule réessaie en Simple, sinon le texte est collé sans mise au propre. S’il en laisse passer une avec un mode personnel, ajoute à ce mode un exemple de question.')}
      {@render question('Whisper écrit « Mardi » ou « Vendredi » avec une majuscule.', 'En français les jours s’écrivent sans majuscule, mais Whisper les écrit parfois à l’anglaise. Ce n’est qu’une question de présentation : le sens ne change pas. Quand un modèle de mise au propre est actif, il corrige ce défaut ; sans lui (mode Littéral, mise au propre « Aucune »), la majuscule reste. Pour l’éviter, ajoute dans le Dictionnaire une correction automatique « mardi » → « mardi » (une par jour) : elle remplace toute écriture par la tienne, majuscule comprise. Attention, elle met aussi le jour en minuscule en début de phrase.')}
      {@render question('Un mot est toujours mal écrit.', 'Ajoute-le dans « Mes mots », ou une correction automatique (ce que Whisper entend → ce qu’il doit écrire). Un modèle Whisper plus gros entend mieux mais prend plus de temps : le Banc d’essai permet de le mesurer sur tes phrases.')}
      {@render question('La musique ou la vidéo ne s’arrête pas.', `Vérifie que « Mettre en pause la musique ou la vidéo » est cochée (Réglages → Dictée). Le journal dit ce que Scriibe voit : « rien ne joue » liste les applis audio et leur niveau, « pause demandée » dit à qui la commande est partie. Pendant un appel, rien n’est mis en pause, exprès.`)}
      {@render question('La fenêtre est vide ou ne s’ouvre pas.', mac ? 'Clique sur l’icône dans la barre des menus → « Ouvrir Scriibe ». Fermer la fenêtre ne quitte pas Scriibe : pour l’arrêter, « Quitter Scriibe ».' : linux ? 'Cherche « Scriibe » dans la liste des applications, ou clique sur son icône en haut à droite → « Ouvrir Scriibe ». Fermer la fenêtre ne quitte pas Scriibe : pour l’arrêter, « Quitter Scriibe ».' : 'Clic droit sur l’icône près de l’horloge → « Ouvrir Scriibe ». Si WebView2 manque, la même interface s’ouvre dans ton navigateur. Fermer la fenêtre ne quitte pas Scriibe : pour l’arrêter, « Quitter Scriibe ».')}
      {@render question('Comment mettre à jour ?', `Réglages → Mise à jour : « Chercher une nouvelle version » dit si une version plus récente que la vôtre est publiée (version, date, source) ; aucun compte ni jeton n’est nécessaire. ${mac ? '« Installer et redémarrer » demande confirmation, télécharge la nouvelle version, remplace l’application Scriibe.app et la relance, en gardant l’ancienne à côté.' : linux ? '« Installer et redémarrer » demande confirmation, télécharge la nouvelle version, remplace le programme scriibe et relance Scriibe, en gardant l’ancien à côté (scriibe.old).' : '« Installer et redémarrer » demande confirmation, télécharge la nouvelle version, remplace scriibe.exe et relance Scriibe, en gardant l’ancien à côté (scriibe.exe.old).'} La recherche automatique (toutes les 6 heures) est décochée au départ. Cette page explique aussi ce qui part sur Internet, c’est-à-dire rien de personnel.`)}
      <div class="links">{@render go('Installation', 'install')}{@render go('Rapport et journal', 'journal')}{@render go('Mise à jour', 'update')}</div>
    {/if}
  </section>
</div>

<style>
  .lead { margin: 0 0 12px; }
  /* Sommaire fixe, seule la carte de texte défile ; jamais de retour à la ligne (sinon le sommaire passerait au-dessus et le sticky recouvrirait le texte). */
  .layout { display: flex; gap: 14px; align-items: flex-start; flex-wrap: nowrap; }
  .toc { list-style: none; margin: 0; padding: 0; width: clamp(140px, 24%, 220px); flex: none; display: flex; flex-direction: column; gap: 2px; position: sticky; top: 20px; max-height: calc(100vh - 40px); overflow-y: auto; }
  .toc button { width: 100%; text-align: left; border: 0; background: transparent; padding: 8px 10px; border-radius: 8px; }
  .toc button:hover { background: var(--line); }
  .toc button.active { background: var(--accent); color: var(--accent-ink); }
  .body { flex: 1 1 0; min-width: 0; margin: 0; max-width: 1100px; }
  /* Fenêtre étroite : la barre principale (210 px) et le sommaire laisseraient moins de 300 px au texte ; le sommaire devient une bande fixe en haut. */
  @media (max-width: 860px) {
    .layout { flex-direction: column; align-items: stretch; gap: 8px; }
    .toc { flex-direction: row; width: auto; max-height: none; overflow-x: auto; overflow-y: hidden; top: 0; z-index: 5; padding: 6px 0; background: var(--bg); }
    .toc li { flex: none; }
    .toc button { width: auto; white-space: nowrap; }
    .body { flex: none; }
  }
  .body h3 { margin: 18px 0 6px; font-size: 1rem; }
  .body p, .body li { line-height: 1.5; }
  .body ul, .body ol { padding-left: 1.3rem; display: flex; flex-direction: column; gap: 8px; }
  .example { margin: 10px 0; padding: 10px 12px; border-radius: 10px; background: var(--bg); }
  .example .line { display: flex; gap: 10px; align-items: baseline; white-space: pre-line; }
  .example .who { flex: none; width: 96px; font-size: 0.78rem; font-weight: 600; color: var(--muted); }
  .example .who.strong { color: var(--accent); }
  .example .note { margin: 4px 0 0 106px; font-size: 0.85rem; color: var(--muted); }
  .tip { margin: 12px 0; padding: 10px 12px; border-radius: 10px; background: color-mix(in srgb, var(--accent) 12%, transparent); }
  .q { margin: 12px 0; }
  .q > div { margin-top: 2px; line-height: 1.5; }
  .links { display: flex; gap: 8px; flex-wrap: wrap; margin-top: 14px; }
  .go { font-size: 0.9rem; }
  .samples { display: flex; gap: 10px; flex-wrap: wrap; margin: 10px 0; }
  .sample { display: flex; flex-direction: column; align-items: center; gap: 4px; }
  .stagebox { min-width: 150px; min-height: 64px; display: flex; align-items: center; justify-content: center; border-radius: 10px; background: var(--stage); }
</style>
