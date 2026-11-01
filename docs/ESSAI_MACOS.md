# Essai de Scriibe sur macOS — étape 7 (S09 à S11)

**Écrit sans Mac sous la main** (sur le PC Windows de Christophe, le 31/10/2026), puis **essayé le jour même sur son Mac Studio** : compilation, touche, dictée en ligne et collage marchent (détail dans `CHANGELOG.md`, S09). **S10 : appli active, son de fin, traduction, annulation, plusieurs dictées de suite et Whisper local (`small`, 1,6 s pour 11,5 s de parole) marchent aussi.** Lancer Scriibe **depuis son propre Terminal** : voir `docs/PIEGES.md` (autorisations). Une étape à la fois, et on note ce qui résiste dans `docs/PIEGES.md`.

## Ce qui est écrit (`crates/platform/src/macos/`)

- **Touche globale** : un « tap » d'événements de CoreGraphics en **écoute seule** (rien n'est retenu : l'appli active reçoit les touches comme avant). Option, Commande, Ctrl, Maj et Fn arrivent comme changements de modificateurs, **gauche et droite distinguées** (bit propre à chaque touche) ; la touche par défaut est **Option droite**, celle de Scribe. Les touches ordinaires (F1 à F20…) sont ramenées aux codes de Windows (`apple.rs`, testé partout).
- **Collage** : presse-papiers (`arboard`) + **⌘V** envoyé par CoreGraphics, attente du relâchement de Maj, Ctrl, Option et Commande (sinon ⌘V part avec Option), puis l'ancien texte est remis. La touche V est cherchée à sa place QWERTY (code 9), qui est aussi celle de l'AZERTY du Mac ; Dvorak ou Colemak la déplacent (limite connue).
- **Appli au premier plan** : `lsappinfo front` puis `lsappinfo info` (nom et identifiant de bundle), sans autorisation à demander.
- **Micro** : `cpal` (CoreAudio). **Sons** : synthétisés, joués par `afplay`. **Clés d'API** : trousseau de macOS (service « Scriibe », comptes `deepgram`, `openrouter`, `github`), sinon les variables d'environnement. **Parcourir…** (modèle Whisper) : boîte `choose file` d'`osascript`. **Langue du système** : `defaults read -g AppleLocale`.
- Les moteurs savent déjà où chercher sur macOS (`brew install whisper-cpp`, `brew install llama.cpp`) et le dossier de données est `~/Library/Application Support/Scriibe`.

## Pas encore fait

- **Pause des médias** : faite en S10 (`macos/media.rs`), essayée avec Chrome seulement ; l'écriture de Scriibe au journal (« médias: … ») dit ce qu'elle voit.
- **Fait en S11** : fenêtre Tauri (WKWebView), icône de la barre des menus et son menu, pastille, verrou d'instance (`flock`), Scriibe.app (voir ci-dessous).
- **Fait en S13 (écrit, pas encore essayé sur le Mac)** : touches réglables (Réglages → Dictée → « Changer… ») : Option, ⌘, Fn, Ctrl, Maj (gauche et droite), F1 à F20, Début, Fin, Page préc., Page suiv. (`keys::MAC_CHOICES`, `keys::MAC_LAYOUT`).
- **Pas fait** : **détection de la carte graphique** : à écrire (`system_profiler`) ; **mise à jour intégrée** et **téléchargement des moteurs** par la page : Windows seulement (sur Mac, passer par Homebrew).

## Scriibe.app (S11)

`tools/faire_app_macos.sh` (après `cd ui && npm run build` et `cargo build --release`) fabrique `Scriibe.app`, la signe (« Scribe Dev », identifiant `fr.scriibe.app`) et l'installe dans `~/Applications`. Double-clic : la fenêtre s'ouvre, l'icône apparaît dans la barre des menus (pas d'icône dans le Dock). Les autorisations (Surveillance de l'entrée, Micro, Contrôle de l'appareil et accès aux données pour ⌘V) se donnent **à Scriibe.app**, une fois, puis relancer ; elles tiennent d'une compilation à l'autre. Fermer la fenêtre ne quitte pas ; un second double-clic la remontre ; « Quitter Scriibe » est dans le menu de l'icône. Fermer d'abord un ancien Scriibe lancé depuis un Terminal. Les étapes ci-dessous valent pour un lancement depuis un terminal (`target/release/scriibe lancer`) ; dans ce cas c'est le terminal qui reçoit les autorisations.

## Avant de commencer

1. **Quitter Scribe** (la version Swift) : elle écoute aussi Option droite, les deux collent chacune leur texte.
2. Outils : `xcode-select --install`, [Rust](https://rustup.rs), Node.js (pour construire l'interface).

## Essayer (une étape à la fois)

1. Construire : `git pull`, `cd ui && npm ci && npm run build && cd ..`, `cargo build --release` ; exécutable `target/release/scriibe`. **Premier point à rapporter : les erreurs de compilation, s'il y en a** (le code n'a jamais été compilé pour de bon sur un Mac).
2. `target/release/scriibe verifier` : ce que Scriibe trouve (moteurs, micro).
3. Clés des moteurs en ligne, pour un premier essai **sans rien installer** : `scriibe cle deepgram` puis `scriibe cle openrouter` (la clé est lue sur l'entrée et rangée dans le trousseau ; macOS peut demander le mot de passe de la session).
4. `target/release/scriibe lancer` : macOS demande les autorisations, à accorder **au terminal** (Réglages Système → Confidentialité et sécurité), puis **relancer Scriibe** :
   - **Surveillance de l'entrée** (la touche globale) ; si elle manque, Scriibe le dit au démarrage ;
   - **Microphone** ;
   - **Accessibilité** (envoyer ⌘V) ; sans elle, le texte reste dans le presse-papiers. Scriibe fait afficher la demande de macOS au premier collage refusé. **Sur un macOS récent (français), cette autorisation s'appelle « Contrôle de l'appareil et accès aux données »** : la rubrique « Accessibilité » des Réglages Système est autre chose (vision, audition…).
5. Dans TextEdit ou Notes : maintenir **Option droite**, parler, relâcher. Rapporter : le texte collé ? le son de début et de fin ? la ligne « appli : … » du journal (nom de l'appli) ? Avec **⌘ gauche** tenue en plus : traduction.
6. « Copier le rapport » (Journal et rapport) et le coller dans la session.

Whisper en local : `brew install whisper-cpp`, un modèle `ggml-*.bin` dans `~/Library/Application Support/Scriibe/models/` (ou « Parcourir… » dans Réglages), puis choisir Whisper dans Réglages → Moteurs.
