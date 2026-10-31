# Essai de Scriibe sur macOS — étape 7, premier jalon (S09)

**Écrit sans Mac sous la main** (sur le PC Windows de Christophe, le 31/10/2026) : le code est vérifié à la compilation croisée et à la lecture, **jamais exécuté**. Le premier essai sur un Mac est donc aussi le premier contact avec le système : on s'attend à corriger des choses. Une étape à la fois, et on note ce qui résiste dans `docs/PIEGES.md`.

## Ce qui est écrit (`crates/platform/src/macos/`)

- **Touche globale** : un « tap » d'événements de CoreGraphics en **écoute seule** (rien n'est retenu : l'appli active reçoit les touches comme avant). Option, Commande, Ctrl, Maj et Fn arrivent comme changements de modificateurs, **gauche et droite distinguées** (bit propre à chaque touche) ; la touche par défaut est **Option droite**, celle de Scribe. Les touches ordinaires (F1 à F20…) sont ramenées aux codes de Windows (`apple.rs`, testé partout).
- **Collage** : presse-papiers (`arboard`) + **⌘V** envoyé par CoreGraphics, attente du relâchement de Maj, Ctrl, Option et Commande (sinon ⌘V part avec Option), puis l'ancien texte est remis. La touche V est cherchée à sa place QWERTY (code 9), qui est aussi celle de l'AZERTY du Mac ; Dvorak ou Colemak la déplacent (limite connue).
- **Appli au premier plan** : `lsappinfo front` puis `lsappinfo info` (nom et identifiant de bundle), sans autorisation à demander.
- **Micro** : `cpal` (CoreAudio). **Sons** : synthétisés, joués par `afplay`. **Clés d'API** : trousseau de macOS (service « Scriibe », comptes `deepgram`, `openrouter`, `github`), sinon les variables d'environnement. **Parcourir…** (modèle Whisper) : boîte `choose file` d'`osascript`. **Langue du système** : `defaults read -g AppleLocale`.
- Les moteurs savent déjà où chercher sur macOS (`brew install whisper-cpp`, `brew install llama.cpp`) et le dossier de données est `~/Library/Application Support/Scriibe`.

## Pas encore fait

- **Pause des médias** (`NoMedia`) : il faudra CoreAudio (quels processus jouent ou captent le micro, macOS 14.4 et après) et la touche Lecture/Pause du clavier.
- **Icône de la barre des menus** et **fenêtre de Scriibe** : l'interface s'ouvre dans le navigateur (`open`). La fenêtre Tauri (WKWebView) et l'icône sont à porter.
- **Verrou d'instance** (une seule Scriibe à la fois) : Windows seulement ; **détection de la carte graphique** : à écrire (`system_profiler`) ; **mise à jour intégrée** et **téléchargement des moteurs** par la page : Windows seulement (sur Mac, passer par Homebrew).
- Pas de `.app` : Scriibe se lance depuis un terminal, et c'est **le terminal** qui reçoit les autorisations ci-dessous.

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
   - **Accessibilité** (envoyer ⌘V) ; sans elle, le texte reste dans le presse-papiers.
5. Dans TextEdit ou Notes : maintenir **Option droite**, parler, relâcher. Rapporter : le texte collé ? le son de début et de fin ? la ligne « appli : … » du journal (nom de l'appli) ? Avec **⌘ gauche** tenue en plus : traduction.
6. « Copier le rapport » (Journal et rapport) et le coller dans la session.

Whisper en local : `brew install whisper-cpp`, un modèle `ggml-*.bin` dans `~/Library/Application Support/Scriibe/models/` (ou « Parcourir… » dans Réglages), puis choisir Whisper dans Réglages → Moteurs.
