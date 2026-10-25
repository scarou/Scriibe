# Écarts voulus avec Scribe

Tout écart de comportement entre Scriibe et la version de référence (Scribe, macOS) se note ici, avec la raison et le cas de `fixtures/` concerné. Un cas de référence ne se modifie jamais en silence.

| Date | Sujet | Écart | Raison | Cas |
|---|---|---|---|---|
| 2026-10-25 | Nom de l'appli dans les messages | « lancé par Scriibe » au lieu de « lancé par Scribe » (`llama::status`) | L'appli s'appelle Scriibe | `local_engines-036` à `-038` (le texte testé ne contient pas le nom) |
| 2026-10-25 | Commande d'installation de `llama-server` | Passée en paramètre (`install_hint`) au lieu de « brew install llama.cpp » en dur | Le système change (winget, apt, brew…) | `local_engines-034` |
| 2026-10-25 | Reconnaissance du français dans le garde-fou de la traduction | `lingua` à la place de `NLLanguageRecognizer` (Apple) | Pas d'équivalent portable ; mêmes seuils (français > 0,5 et > langue cible) | `translation_guard-*` (tous passent) |
| 2026-10-25 | Mode Amusant (schtroumpf) | La mise au propre se fait en Simple, sans la transformation schtroumpf | Le moteur dépend de l'analyseur grammatical d'Apple (reporté) | `playful_schtroumpf-*` (reportés) ; `playful_mode_is_cleaned_in_simple_for_now` |
| 2026-10-25 | Arrêt d'un serveur | Arrêt immédiat du processus (puis attente de sa fin), sans SIGTERM puis SIGKILL 0,3 s plus tard | `std` ne sait pas envoyer SIGTERM ; Windows n'a pas d'équivalent ; le serveur n'a rien à sauvegarder | `server.rs` |
| 2026-10-25 | Fils de calcul de Whisper | `min(8, cœurs)` au lieu de 8 fixes | Un ordinateur à 4 cœurs ne doit pas en saturer 8 | `whisper_starts_once_and_is_reused` |
| 2026-10-25 | En-tête `X-Title` envoyé à OpenRouter | « Scriibe » au lieu de « Scribe » | Nom de l'appli | `openrouter_needs_a_key_and_sends_it` |
| 2026-10-25 | Palmarès des applis (`top_apps`) | À égalité de mots, ordre alphabétique | Scribe laisse l'ordre au hasard du dictionnaire | — |
| 2026-10-25 | Libellé de l'indicateur de micro Bluetooth | Inchangé (« — Bluetooth, déconseillé ») | — | `microphone-010` |

Différences de détail connues, sans effet mesuré : Swift compte des *grapheme clusters* (`Character`) là où Rust compte des *points de code* (`char`) pour les longueurs et les tests « est une lettre » ; les accents décomposés (NFD) peuvent donc changer une longueur de 1 ou 2.

## Choix à faire (pas encore des écarts)

- Touche de dictée par défaut sous Windows et Linux : Option droite est AltGr sur un clavier AZERTY.
- Touche de traduction : ⌘ n'existe pas sous Windows/Linux (Ctrl ? Maj ?).
- Mode Amusant (schtroumpf) : reporté, dépend de l'analyse grammaticale d'Apple.
- Identifiants de la pause des médias : ceux de macOS ne valent pas ailleurs (règles gardées, identifiants à remplacer).
