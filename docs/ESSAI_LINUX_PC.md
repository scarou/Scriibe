# Consignes pour une session Claude lancée SUR le PC Linux de l'auteur

Fichier écrit le 03/11/2026 par la session « Version Linux » du Mac, pour la session Claude Code qui tourne **sur le PC Linux** (HP ProBook 450 G6, Ubuntu 26.04.1, bureau GNOME en **Wayland** ; Xfce en X11 est installé en plus). Les deux ordinateurs sont séparés : **L'auteur ne peut rien copier d'un PC à l'autre** et ne comprend ni le jargon ni les chemins abrégés. Lis d'abord `CLAUDE.md` (règles du projet) et `docs/ESSAI_LINUX.md`.

## Règles de conduite avec l'auteur

- Une étape à la fois, une seule question courte par message, en français simple. Fais toi-même tout ce qui est faisable (commandes, lecture des journaux, installation) ; ne lui demande que ce que lui seul peut faire (mot de passe `sudo`, cliquer, tenir la touche et parler). Il ne doit jamais retaper une commande : lance-les toi-même.
- **Principe de l'auteur : le logiciel doit s'adapter à son environnement, pas l'inverse.** La cible finale est donc **GNOME Wayland sur Ubuntu 26.04** (pas besoin de changer de bureau). Xfce/X11 ne sert qu'à vérifier que le reste de la chaîne marche.
- Rien n'est téléchargé sans son accord (annonce la source et la taille). Pas de publication de Release.

## Ce qui existe déjà (voir `CHANGELOG.md`, `docs/ESSAI_LINUX.md`)

- Couche X11 (`crates/platform/src/linux/`) : touche globale par XInput2 (Ctrl droit par défaut), collage (presse-papiers + Ctrl+V par XTest), micro `cpal` (ALSA), sons, pause des médias. **Rien de cela n'a jamais été essayé sur un vrai bureau.** Wayland est refusé avec un message.
- La page de Scriibe s'ouvre dans le navigateur (pas d'icône, pas de fenêtre propre).
- Moteurs : l'onglet Installation (Réglages → Moteurs) propose sous Linux `whisper-server (Linux)` et `llama-server (Linux, processeur)`, fichiers que `release-linux.yml` ajoute à chaque Release. **Ils ne sont dans une Release qu'à partir de la prochaine version publiée** (la Release v0.2.2 n'a que `scriibe-linux-x64.zip`, sans ces cartes) : pour l'essai, **compile Scriibe depuis les sources** sur ce PC.

## Étapes

1. **Compiler** (le Mac a déjà tout vérifié ; CI verte sur les trois systèmes) : `git clone https://github.com/scarou/Scriibe`, installer ce qui manque (`build-essential`, `libasound2-dev`, `git`, `curl`, Rust par rustup, Node 22), puis `cd ui && npm ci && npm run build`, `cargo build --release -p scriibe-cli`. Demande son mot de passe à l'auteur une seule fois pour les `sudo`. Annonce la taille des téléchargements.
2. **Lancer** `target/release/scriibe lancer --sans-navigateur` avec le journal redirigé dans un fichier que **tu** lis ; ouvre la page dans le navigateur. Les lignes `ALSA lib pcm_dsnoop… unable to open slave` sont (a priori) du bruit de `cpal` : vérifie que le micro s'ouvre vraiment (Réglages → Mesures et essais).
3. **Moteurs** : demande à l'auteur de cliquer sur les cartes de téléchargement (si elles ne trouvent rien parce qu'aucune Release ne les porte encore, compile `whisper-server` toi-même : `docs/ESSAI_LINUX.md`, section « Compiler `whisper-server` soi-même »). Choisir Whisper ; la mise au propre peut rester éteinte au premier essai.
4. **Essai en session Xfce/X11** (déconnexion puis roue dentée de l'écran de connexion → « Xfce Session ») : Ctrl droit maintenu, une phrase dans un éditeur de texte, relâcher. Note chaque résultat dans `docs/ESSAI_LINUX.md` (section « Résultats ») et chaque piège dans `docs/PIEGES.md`.
5. **Wayland (GNOME)**, le vrai objectif : étudie ce que le bureau offre, *sur place* (c'est le seul endroit où on peut le tester) : portail `org.freedesktop.portal.GlobalShortcuts` (signaux `Activated` / `Deactivated` = appui / relâchement ; GNOME 48+), portail `org.freedesktop.portal.RemoteDesktop` (`NotifyKeyboardKeycode` pour coller), repli « copie seule » (le texte reste dans le presse-papiers). Écris un module `crates/platform/src/linux/wayland.rs` derrière l'interface existante (`crates/platform`), crate `ashpd` ou `zbus`. La combinaison « Ctrl droit seul » ne sera probablement pas permise par le portail : prévois de la choisir dans la fenêtre de GNOME et de l'expliquer dans la page. Pas de logique système dans `crates/core`.
6. Règles du projet avant tout commit : `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, et dans `ui` : `npm run check && npm test && npm run build`. Tout texte de la page passe par `t('…')` avec sa traduction dans `ui/src/lib/locales` ; `Help.svelte` ↔ `HelpEn.svelte` ; guides `.md` ↔ `.en.md`.

## Pousser le travail

Les commits se font à l'identité de l'auteur (déjà réglée dans Git sur la machine ; ne jamais écrire l'adresse dans un fichier du dépôt), message court en français, ligne `Co-Authored-By`. Pour pousser, il faut que GitHub reconnaisse ce PC : propose à l'auteur `gh auth login` (navigateur, un code à saisir : c'est le seul « copier » qu'il aura à faire) et pousse sur une **branche `linux-pc`**, pas sur `main`. La session du Mac relira cette branche. Si l'auteur préfère ne pas connecter GitHub, laisse les commits en local et résume en quelques lignes, à lui lire à voix haute, ce qui marche et ce qui bloque.
