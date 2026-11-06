# Scriibe sur Mac — guide pour connaisseurs

🇬🇧 [English version](INSTALLATION_MACOS_EXPERT.en.md)

Autres guides : [installation pas à pas pour débutants](INSTALLATION_MACOS.md) · [Windows](INSTALLATION_WINDOWS.md) · [Linux](INSTALLATION_LINUX.md)

Ce guide suppose que vous savez utiliser le Terminal. Il regroupe ce qui ne figure pas dans le guide pour débutants : les moteurs en ligne, Homebrew en détail, la mise à jour à la main, la compilation depuis les sources, la dictée sur une sélection et la publication d'une version.

## Homebrew et les deux moteurs locaux

Scriibe appelle deux programmes externes en HTTP, comme Scribe : `whisper-server` (transcription) et `llama-server` (mise au propre).

```bash
brew install whisper-cpp
brew install llama.cpp
```

Scriibe les cherche dans le `PATH` de Homebrew (`/opt/homebrew/bin` sur puce Apple). La carte **Les programmes** de **Réglages → Moteurs** ne propose pas de téléchargement pour Mac (elle liste des programmes Windows et Linux) : sur Mac, les programmes viennent de Homebrew. Les modèles se téléchargent depuis les cartes **Modèles recommandés** et **Ajouter un modèle depuis Hugging Face**, ou se déposent à la main dans `~/Library/Application Support/Scriibe/models/`. Les fichiers au format GGUF servent à la mise au propre, au format `ggml-….bin` à Whisper.

## Moteurs en ligne (dépannage)

Plus rapides à mettre en place, ou utiles sur une machine modeste, mais payants à l'usage : la voix part chez Deepgram, le texte chez OpenRouter pour être mis au propre. Il faut un compte et une clé pour chacun : collez chaque clé dans **Réglages → Moteurs → Clés du cloud (dépannage)**. Les clés sont rangées dans le trousseau du Mac et jamais affichées.

## Mise à jour à la main

L'installation intégrée (**Réglages → Mise à jour → Chercher une nouvelle version → Installer et redémarrer**) télécharge le zip de la Release, vérifie la signature de la nouvelle application, remplace `Scriibe.app` (l'ancienne reste à côté sous le nom `Scriibe.app.old` jusqu'au lancement suivant) et rouvre Scriibe. À la main, si elle échoue (par exemple `Scriibe.app` n'est pas dans un dossier où vous pouvez écrire) ou depuis une version plus ancienne :

1. Quitter Scriibe : icône de la barre des menus → **Quitter Scriibe**.
2. Télécharger le zip de la version la plus récente sur `https://github.com/scarou/Scriibe/releases` (comparer son numéro à celui de **Réglages → Mise à jour**).
3. Décompresser, glisser **Scriibe** dans **Applications**, **Remplacer**.
4. Rouvrir. Réglages, historique et dictionnaire sont conservés (rangés à part). Les autorisations le sont normalement ; sinon, les redonner (étape 4 du guide pour débutants).

## Autorisations et signature

Scriibe est signé par un certificat local (« Scribe Dev »), pas par un compte développeur Apple : macOS affiche l'avertissement de premier lancement et, après une recompilation signée autrement, peut redemander les autorisations ou l'accès au trousseau. Les trois autorisations (Microphone, Surveillance de l'entrée, Contrôle de l'appareil et accès aux données) vont à **`Scriibe.app`** lancée par `open` ou par le Finder. Un programme lancé depuis un autre processus (par exemple le binaire `target/release/scriibe` lancé par un outil) n'en a aucune : voir `docs/PIEGES.md`.

## Compiler depuis les sources

Prérequis : Rust, Node.js, Xcode Command Line Tools.

```bash
cd ui && npm ci && npm run build && cd ..
cargo build --release
tools/faire_app_macos.sh --lancer
```

Le script assemble `Scriibe.app` depuis `target/release/scriibe`, la signe avec « Scribe Dev » et l'installe dans `~/Applications` (ou dans `SCRIIBE_INSTALL=chemin/Scriibe.app`). Construire `ui/dist` **avant** `cargo build` : sinon l'interface est remplacée par un message « Interface absente ». Le journal est `~/Library/Application Support/Scriibe/scriibe.log`. Voir `docs/ESSAI_MACOS.md`.

## Dictée sur une sélection

Si un mot ou un groupe de mots est sélectionné quand vous relâchez la touche, le texte dicté le remplace **sans majuscule au début ni point, virgule ou point-virgule final**, puisqu'il s'insère au milieu d'une phrase (les points d'interrogation et d'exclamation, les deux-points et les points de suspension restent). Les noms du dictionnaire, les sigles et les mots comme « iPhone » gardent leur majuscule. Cela fonctionne sur Mac ; ce n'est pas encore disponible sous Windows ni Linux.

## Pour le créateur : publier une version

Une version se compile et se signe sur le Mac de Christophe (certificat « Scribe Dev »), puis se publie en une commande :

```bash
tools/publier_macos.sh 0.2.0
```

Le script exige une compilation à jour (`cd ui && npm ci && npm run build`, puis `cargo build --release`), assemble `Scriibe.app`, la zippe, **montre l'aperçu** (nom, version, taille) et ne publie qu'après un « oui ». Il crée une étiquette et une publication (Release) sur `scarou/Scriibe` avec le zip en pièce jointe ; un workflow ajoute ensuite le fichier Windows.
