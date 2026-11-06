# Scriibe sur Windows — guide pour connaisseurs

🇬🇧 [English version](INSTALLATION_WINDOWS_EXPERT.en.md)

Autres guides : [installation pas à pas pour débutants](INSTALLATION_WINDOWS.md) · [macOS](INSTALLATION_MACOS_EXPERT.md) · [Linux](INSTALLATION_LINUX.md)

Ce guide regroupe ce qui ne figure pas dans le guide pour débutants : la version compilée par la CI (voie B), les moteurs en ligne, les moteurs en détail, la mise à jour à la main, la compilation depuis les sources et la publication d'une version. Le détail de ce qui a été essayé sur le PC de l'auteur est dans `docs/ESSAI_WINDOWS.md`.

## Voie B : dernière version compilée par GitHub

Utile entre deux Releases, ou si le zip n'est pas encore attaché à la dernière (il est ajouté par le workflow `release-windows.yml` quelques minutes après la publication).

1. Se connecter à GitHub (compte gratuit : `https://github.com/signup` si besoin ; GitHub ne laisse télécharger ces fichiers qu'aux personnes connectées).
2. Ouvrir `https://github.com/scarou/Scriibe/actions/workflows/ci.yml`.
3. Cliquer sur la première exécution marquée d'une **coche verte**, descendre jusqu'à la rubrique **Artifacts**, cliquer sur **`scriibe-windows-x64`**. Le zip arrive dans **Téléchargements**. GitHub efface ces fichiers après 90 jours : si la liste est vide, demander une nouvelle version.

## Les deux moteurs locaux

Scriibe appelle deux programmes externes en HTTP, comme Scribe : `whisper-server` (transcription) et `llama-server` (mise au propre). La carte **Les programmes** de **Réglages → Moteurs** les télécharge un à un, avec taille et source avant l'accord :

- `whisper-server (Windows)` : whisper.cpp avec ses DLL ;
- `llama-server (Windows, processeur)` ou `llama-server (Windows, carte graphique Vulkan)`. La version Vulkan remplace la version processeur : la mise au propre tourne sur la carte graphique (à comparer aux temps du processeur ; pour revenir en arrière, retélécharger la version processeur). L'assistant du **Tableau de bord** mesure la machine et propose la chaîne adaptée avec des temps mesurés.

Les modèles se téléchargent depuis les cartes **Modèles recommandés** et **Ajouter un modèle depuis Hugging Face**, ou se déposent à la main dans le dossier des modèles de Scriibe. Les fichiers au format GGUF servent à la mise au propre, ceux au format `ggml-….bin` à Whisper (**Réglages → Moteurs → Choisir un fichier…** pour pointer un fichier ailleurs). La mise au propre peut aussi passer par LM Studio ou Ollama (liste des modèles du serveur, adresse réglable).

## Moteurs en ligne (dépannage)

Plus rapides à mettre en place, ou utiles sur une machine modeste, mais payants à l'usage (quelques centimes la dictée) : la voix part chez Deepgram, le texte chez OpenRouter pour être mis au propre. Il faut un compte et une clé pour chacun : collez chaque clé dans **Réglages → Moteurs → Clés du cloud (dépannage)**. Les clés sont rangées dans le Gestionnaire d'identifiants de Windows et jamais affichées.

## Mise à jour à la main

L'installation intégrée (**Réglages → Mise à jour → Chercher une nouvelle version → Installer et redémarrer**) télécharge `scriibe-windows-x64.zip` de la dernière Release (sans compte ni jeton), remplace `scriibe.exe` et relance Scriibe ; l'ancien programme reste à côté sous le nom `scriibe.exe.old` jusqu'au lancement suivant. À la main, si elle échoue ou depuis une version plus ancienne :

1. Quitter Scriibe : clic droit sur l'icône près de l'horloge → **Quitter**.
2. Télécharger le zip de la version la plus récente (Releases, ou voie B ci-dessus).
3. L'extraire **dans le même dossier** (`C:\scriibe`), en acceptant de **remplacer** `scriibe.exe`.
4. Relancer `scriibe.exe`. Réglages, historique et dictionnaire sont conservés (rangés à part).

## Avertissement SmartScreen

Scriibe n'est pas signé (certificat de signature de code payant) : SmartScreen l'annonce au premier lancement (**Informations complémentaires → Exécuter quand même**). Si ce bouton manque : clic droit sur `scriibe.exe` → **Propriétés** → **Débloquer**.

## Touche de dictée

Par défaut **Ctrl droit** maintenue (**Maj** en plus : traduction ; une autre touche : annulation), modifiable dans **Réglages → Dictée** (jusqu'à trois touches). Le crochet clavier de Windows ne reçoit rien quand la fenêtre de Scriibe a le focus : la page relaie alors ses touches ; « Tester ma touche » se fait dans une autre fenêtre. Voir `docs/ESSAI_WINDOWS.md`.

## Compiler depuis les sources

Prérequis : Rust (cible `x86_64-pc-windows-msvc`), Node.js, Git. Dans un dossier de travail (par exemple `C:\scriibe-dev`) :

```powershell
git pull
cd ui; npm ci; npm run build; cd ..
cargo build --release
```

L'exécutable est `target\release\scriibe.exe`. Construire `ui\dist` **avant** `cargo build` : sinon l'interface est remplacée par un message « Interface absente ». Fermer la version en cours avant de recompiler. `C:\scriibe` garde le `scriibe.exe` de la mise à jour par la fenêtre.

## Pour le créateur : publier une version

Le numéro se règle dans `Cargo.toml`, puis `tools/publier_macos.sh x.y.z` (sur le Mac) publie la Release ; le workflow `release-windows.yml` ajoute alors `scriibe-windows-x64.zip`. Attendre qu'il ait fini avant de demander la mise à jour sur un PC.
