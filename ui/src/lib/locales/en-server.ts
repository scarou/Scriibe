/**
 * Traduction anglaise des textes français que le programme (Rust) envoie à la page : noms et résumés des modes, messages, libellés.
 * Même forme que `en.ts`, mais la page ne les écrit pas elle-même : pas de contrôle d'orphelins (`i18n.test.ts` vérifie les `{valeurs}`).
 */
export const enServer: Record<string, string> = {
  'enregistrement impossible : l’emplacement choisi n’est pas accessible en écriture': 'saving failed: the chosen location cannot be written to',
  'mode inconnu': 'unknown mode',
  'aucun dossier d’enregistrement trouvé': 'no folder to save in was found',

  // Noms des modes (modes.rs)
  'Littéral': 'Literal',
  'Simple': 'Simple',
  'Soutenu': 'Formal',
  'Pro · Sophrologie et bien-être': 'Pro · Sophrology and wellbeing',
  'Pro · Informatique et IA': 'Pro · IT and AI',
  'Pro · Courrier administratif': 'Pro · Official letters',
  'Pro · Commercial': 'Pro · Sales',
  'Amusant (schtroumpf)': 'Playful (Smurf)',
  'Apaisé (CNV)': 'Calmed (NVC)',

  // Résumés des modes (modes.rs)
  'Aucune reformulation : signes dictés, dictionnaire et typographie seulement': 'No rephrasing: dictated signs, dictionary and typography only',
  'Retire les « euh », met la ponctuation, garde tes mots': 'Removes the “uh”s, adds punctuation, keeps your words',
  'Français soutenu, tournures familières reformulées': 'Formal French, colloquial phrasing reworded',
  'Ton posé et bienveillant, vocabulaire de la sophrologie': 'Calm, caring tone, sophrology vocabulary',
  'Précis et concis, termes techniques gardés en anglais': 'Precise and concise, technical terms kept in English',
  'Neutre et poli, destinataire vouvoyé': 'Neutral and polite, recipient addressed formally',
  'Engageant et clair, orienté client': 'Engaging and clear, customer-focused',
  'Pour les échanges tendus : réécrit en communication non violente (faits, ressenti, besoin, demande), sans accuser':
    'For tense exchanges: rewrites in nonviolent communication (facts, feeling, need, request), without blaming',
  'Pour rire : le texte mis au propre (Simple), puis passé en langage schtroumpf sur cet ordinateur':
    'Just for fun: the cleaned-up text (Simple), then turned into Smurf language on this computer',

  // Programmes et modèles à télécharger (downloads.rs)
  'Le programme de transcription (whisper.cpp), avec ses DLL.': 'The transcription program (whisper.cpp), with its DLLs.',
  'whisper-server (Linux)': 'whisper-server (Linux)',
  'Le programme de transcription (whisper.cpp), compilé pour Linux x86-64 par la CI de Scriibe (un seul fichier, processeur avec AVX2).':
    'The transcription program (whisper.cpp), built for Linux x86-64 by Scriibe’s CI (a single file, AVX2 processor).',
  'llama-server (Linux, processeur)': 'llama-server (Linux, CPU)',
  'Le programme de mise au propre (llama.cpp), version processeur pour Linux x86-64 (fichier officiel de llama.cpp, remballé en zip par la CI de Scriibe).':
    'The cleanup program (llama.cpp), CPU version for Linux x86-64 (official llama.cpp file, repacked as a zip by Scriibe’s CI).',
  'Modèle Whisper « base »': 'Whisper “base” model',
  'Le plus rapide ; quelques erreurs sur les mots courts.': 'The fastest; a few mistakes on short words.',
  'Modèle Whisper « small »': 'Whisper “small” model',
  'Plus précis, plus lent.': 'More accurate, slower.',
  'llama-server (Windows, processeur)': 'llama-server (Windows, CPU)',
  'Le programme de mise au propre (llama.cpp), version processeur.': 'The cleanup program (llama.cpp), CPU version.',
  'llama-server (Windows, carte graphique Vulkan)': 'llama-server (Windows, Vulkan graphics card)',
  'Remplace la version processeur : la mise au propre tourne sur la carte graphique (à comparer aux temps du processeur). Pour revenir en arrière, retélécharger la version processeur.':
    'Replaces the CPU version: cleanup runs on the graphics card (compare with the CPU times). To go back, download the CPU version again.',
  'Modèle de mise au propre « Qwen 2.5 0,5 B »': 'Cleanup model “Qwen 2.5 0.5 B”',
  'Très petit (≈ 400 Mo) : le plus rapide sur processeur, à essayer avant de juger sa qualité (Apache 2.0).':
    'Very small (≈ 400 MB): the fastest on CPU, try it before judging its quality (Apache 2.0).',
  'Modèle de mise au propre « Qwen 2.5 1,5 B »': 'Cleanup model “Qwen 2.5 1.5 B”',
  'Petit modèle d\'instructions (licence Apache 2.0), ≈ 1 Go.': 'Small instruction model (Apache 2.0 licence), ≈ 1 GB.',
  'Modèle de mise au propre « Qwen 2.5 3 B »': 'Cleanup model “Qwen 2.5 3 B”',
  'Deux fois plus gros, meilleur, plus lent.': 'Twice as big, better, slower.',

  // Remarques sur les fichiers Hugging Face (hub.rs)
  'modèle en plusieurs fichiers : pas encore pris en charge': 'model split across several files: not supported yet',
  'assistant de décodage (MTP) : à mettre à côté du modèle du même nom': 'decoding assistant (MTP): put it next to the model with the same name',
  'partie « vision » : inutile pour dicter': '“vision” part: not needed for dictation',

  // Chaînes de l'assistant (assistant.rs)
  'En dépannage : Whisper sur cet ordinateur + mise au propre OpenRouter (cloud)': 'As a fallback: Whisper on this computer + OpenRouter cleanup (cloud)',
  'Pour un ordinateur trop lent pour la mise au propre locale. La voix reste sur l\'ordinateur ; seul le texte transcrit part au cloud pour être mis au propre (service payant, clé nécessaire).':
    'For a computer too slow for local cleanup. Your voice stays on the computer; only the transcribed text goes to the cloud to be cleaned up (paid service, key needed).',
  '3,2 à 3,9 s après le relâchement (mesuré sur un portable à 4 cœurs sans carte graphique)':
    '3.2 to 3.9 s after release (measured on a 4-core laptop with no graphics card)',
  'Tout sur cet ordinateur, sans mise au propre': 'Everything on this computer, without cleanup',
  'Whisper seul : le texte est collé tel que transcrit (signes dictés, dictionnaire et typographie appliqués). Aucun compte ; votre voix et votre texte restent sur l\'ordinateur.':
    'Whisper alone: the text is pasted as transcribed (dictated signs, dictionary and typography applied). No account; your voice and text stay on the computer.',
  '2,3 à 3,1 s (mesuré sur un portable à 4 cœurs sans carte graphique)': '2.3 to 3.1 s (measured on a 4-core laptop with no graphics card)',
  'Tout sur cet ordinateur, avec un petit modèle de mise au propre': 'Everything on this computer, with a small cleanup model',
  'Whisper + Qwen 2.5 1,5 B (llama.cpp). Votre voix et votre texte restent sur l\'ordinateur, mais un petit modèle garde parfois les « euh » et peut changer un mot.':
    'Whisper + Qwen 2.5 1.5 B (llama.cpp). Your voice and text stay on the computer, but a small model sometimes keeps the “uh”s and may change a word.',
  '4 à 6 s (mesuré sur un portable à 4 cœurs sans carte graphique) ; 15 à 45 s pour la première dictée après une libération du modèle':
    '4 to 6 s (measured on a 4-core laptop with no graphics card); 15 to 45 s for the first dictation after the model has been released',
  'Tout sur cet ordinateur, avec sa carte graphique': 'Everything on this computer, using its graphics card',
  'Whisper « small » + Qwen 2.5 3 B sur la carte graphique (llama.cpp Vulkan). Sans doute plus rapide et plus juste que sur le processeur.':
    'Whisper “small” + Qwen 2.5 3 B on the graphics card (llama.cpp Vulkan). Probably faster and more accurate than on the CPU.',
  'pas encore mesuré sur une carte de cette classe': 'not yet measured on a card of this class',

  // Niveau du micro (web/mod.rs)
  'muet': 'silent',
  'faible': 'weak',

  // Langues de traduction (translation.rs)
  'anglais': 'English',
  'espagnol': 'Spanish',
  'allemand': 'German',
  'italien': 'Italian',
  'portugais': 'Portuguese',

  // Messages de la pastille (controller.rs)
  'Aucun son capté — vérifie le micro ({name})': 'No sound captured — check the microphone ({name})',
  'Rien entendu — micro faible : rapproche-toi': 'Nothing heard — weak microphone: move closer',
  'Rien entendu': 'Nothing heard',

  // Commandes vocales sur du texte sélectionné (controller.rs, prefill.rs)
  'Ajouter au dictionnaire — complète « Écrit »': 'Add to dictionary — fill in “Written”',
  'Sélection trop longue pour le dictionnaire (80 signes au plus, sur une seule ligne)':
    'Selection too long for the dictionary (80 characters at most, on a single line)',
  'Raccourci invalide dans les réglages (exemple : Mod+B)': 'Invalid shortcut in the settings (example: Mod+B)',
  'Raccourci non envoyé — {reason}': 'Shortcut not sent — {reason}',
  'autorisation Accessibilité manquante': 'Accessibility permission missing',
  'touche non prise en charge': 'key not supported',
  'macOS n\'a pas pu envoyer la touche': 'macOS could not send the key',
  'non pris en charge sur ce système': 'not supported on this system',
  'Rien à entourer (la sélection est vide)': 'Nothing to wrap (the selection is empty)',
  'Scriibe n\'a pas pu ouvrir sa fenêtre': 'Scriibe could not open its window',
  'Commande non faite : autorisation Accessibilité manquante — phrase collée comme du texte':
    'Command not done: Accessibility permission missing — phrase pasted as text',
  'Commande non faite : le presse-papiers contient autre chose que du texte — phrase collée comme du texte':
    'Command not done: the clipboard holds something other than text — phrase pasted as text',
  'Commande non faite : sélection illisible — phrase collée comme du texte': 'Command not done: selection unreadable — phrase pasted as text',
  'Copié, à coller avec Ctrl+V — {reason}': 'Copied, paste it with Ctrl+V — {reason}',
  'Copié, à coller avec ⌘V — {reason}': 'Copied, paste it with ⌘V — {reason}',
  'Collé — micro faible : rapproche-toi ou monte le volume d\'entrée': 'Pasted — weak microphone: move closer or raise the input volume',
  'Collé en {language} — micro faible : rapproche-toi ou monte le volume d\'entrée':
    'Pasted in {language} — weak microphone: move closer or raise the input volume',
  'Traduction écartée — collé en français': 'Translation rejected — pasted in French',
  'Collé en français, sans traduction ({reason})': 'Pasted in French, without translation ({reason})',
  'Collé sans mise au propre ({reason})': 'Pasted without cleanup ({reason})',
  'Reformulation écartée': 'Rewrite rejected',

  // Noms des touches (keys.rs)
  'Option droite': 'Right Option',
  'Option gauche': 'Left Option',
  'Commande droite (⌘)': 'Right Command (⌘)',
  'Commande gauche (⌘)': 'Left Command (⌘)',
  '⌘': '⌘',
  'Ctrl droit': 'Right Ctrl',
  'Ctrl gauche': 'Left Ctrl',
  'Alt gauche': 'Left Alt',
  'Maj gauche': 'Left Shift',
  'Maj droit': 'Right Shift',
  'Maj': 'Shift',
  'Windows gauche': 'Left Windows',
  'Windows droite': 'Right Windows',
  '< > (à gauche de W)': '< > (left of W)',
  'Verr. défil.': 'Scroll Lock',
  'Impr. écran': 'Print Screen',
  'Menu contextuel': 'Context menu',
  'Inser': 'Insert',
  'Début': 'Home',
  'Fin': 'End',
  'Page préc.': 'Page Up',
  'Page suiv.': 'Page Down',
  'Espace': 'Space',
  'Entrée': 'Enter',
  'AltGr (Alt droit)': 'AltGr (right Alt)',
  'une touche inconnue': 'an unknown key',
  'la touche de code {code}': 'the key with code {code}',
  'la touche {letter}': 'the {letter} key',

  // Refus d'une combinaison de touches (keys.rs)
  'trois touches au plus': 'three keys at most',
  'touche inconnue : {id}': 'unknown key: {id}',
  '{label} choisie deux fois': '{label} chosen twice',
  'une seule touche autre que Ctrl, Alt, Maj ou Windows dans la combinaison':
    'only one key other than Ctrl, Alt, Shift or Windows in the combination',
  'une seule touche autre que Ctrl, Maj, Option, ⌘ ou Fn dans la combinaison':
    'only one key other than Ctrl, Shift, Option, ⌘ or Fn in the combination',
  'la dictée utilise ⌘ : choisis une autre touche pour traduire': 'dictation uses ⌘: choose another key for translation',
  'la dictée utilise Maj : choisis une autre touche pour traduire': 'dictation uses Shift: choose another key for translation',
  '{label} sert déjà à dicter': '{label} is already used to dictate',
  'une seule touche pour traduire': 'only one key to translate',

  // Banc de dictée français : catégories (bench_phrases.json)
  'Propre': 'Clean',
  'Propre, longue': 'Clean, long',
  'Question': 'Question',
  'Hésitations': 'Hesitations',
  'Hésitation en tête': 'Hesitation at the start',
  'Reprise': 'Self-correction',
  'Reprise d\'une traite': 'Self-correction in one go',
  'Reprise courte': 'Short self-correction',
  'Reprise courte d\'une traite': 'Short self-correction in one go',
  'Répétition': 'Repetition',
  'Signes dictés': 'Dictated signs',
  'Question à plat': 'Flat question',
  'Question sans inversion': 'Question without inversion',
  'Nombres': 'Numbers',
  'Oral familier': 'Casual speech',
  'Longue': 'Long',

  // Banc de dictée français : ce qu'on attend
  'Whisper la rend propre : sautée si l\'option est active.': 'Whisper returns it clean: skipped if the option is on.',
  'Propre aussi, plus longue : c\'est là que sauter le modèle fait gagner le plus.':
    'Clean too, and longer: this is where skipping the model saves the most.',
  'Lis-la comme une question : elle doit rester une question.': 'Read it as a question: it must stay a question.',
  'Dis les « euh » : s\'ils restent dans ce que Whisper a entendu, le modèle doit être appelé.':
    'Say the “uh”s: if they stay in what Whisper heard, the model has to be called.',
  '« Bon ben » demande le modèle.': '“Bon ben” needs the model.',
  'Le texte collé ne doit garder que onze heures.': 'The pasted text must keep only eleven o\'clock.',
  'Lis-la d\'une traite, sans pause : si elle est sautée et que « dix heures » reste, c\'est un saut à tort.':
    'Read it in one go, without a pause: if it is skipped and “dix heures” stays, that is a wrong skip.',
  'Le texte collé ne doit garder que mardi.': 'The pasted text must keep only Tuesday.',
  'D\'une traite : le texte collé ne doit garder que mardi.': 'In one go: the pasted text must keep only Tuesday.',
  'Le « je » répété doit disparaître.': 'The repeated “je” must disappear.',
  'Parenthèses et retour à la ligne attendus dans le texte collé.': 'Parentheses and a line break expected in the pasted text.',
  'Lis-la sans intonation : le « ? » doit apparaître.': 'Read it without intonation: the “?” must appear.',
  'Lis-la comme une question : si elle est sautée sans « ? », c\'est un saut à tort.':
    'Read it as a question: if it is skipped without a “?”, that is a wrong skip.',
  '« Claude Code » doit être écrit ainsi : s\'il ne l\'est pas, ajoute-le au dictionnaire (comme tes autres mots).':
    '“Claude Code” must be written like this: if it is not, add it to the dictionary (like your other words).',
  'Montant et date justes, en chiffres ou en lettres.': 'Amount and date right, in digits or in words.',
  'Ton familier : « le truc » doit rester. Le modèle retire « tu vois » (tic de langage) ; sautée, la phrase le garde.':
    'Casual tone: “le truc” must stay. The model removes “tu vois” (a verbal tic); when skipped, the phrase keeps it.',
  'Une vingtaine de secondes : regarde surtout le temps total.': 'About twenty seconds: look mostly at the total time.',

  // Contrôles du banc et de l'essai d'un mode
  'reste une question (« ? »)': 'stays a question (“?”)',
  'retour à la ligne': 'line break',
  'heure gardée': 'time kept',
  '« dîner ensemble » gardé': '“dîner ensemble” kept',
  'participants gardés': 'participants kept',
  '« prochaine séance » gardée': '“prochaine séance” kept',
  '« devis » gardé': '“devis” kept',
  '« euh » retirés': '“euh” removed',
  '« jeudi » gardé': '“jeudi” kept',
  '« bon ben » retiré': '“bon ben” removed',
  '« demain matin » gardé': '“demain matin” kept',
  'onze heures gardé': 'eleven o\'clock kept',
  'dix heures retiré': 'ten o\'clock removed',
  '« non pardon » retiré': '“non pardon” removed',
  'mardi gardé': 'Tuesday kept',
  'lundi retiré': 'Monday removed',
  '« je » répété retiré': 'repeated “je” removed',
  '« colis » gardé': '“colis” kept',
  'parenthèses': 'parentheses',
  'signes pas écrits en toutes lettres': 'signs not written out in words',
  '« Claude Code » écrit ainsi': '“Claude Code” written like this',
  '« skill » gardé': '“skill” kept',
  'montant juste': 'amount right',
  'date juste': 'date right',
  '« le truc » gardé': '“le truc” kept',
  '« pas le temps » gardé': '“pas le temps” kept',
  '« respiration abdominale » gardée': '“respiration abdominale” kept',
  '« lieu ressource » gardé': '“lieu ressource” kept',
  'cinq minutes gardées': 'five minutes kept',
  '« tu vois » retiré': '“tu vois” removed',
  '« annexes » gardées': '“annexes” kept',
};
