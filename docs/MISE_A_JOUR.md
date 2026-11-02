# Cadrage — mise à jour de Scriibe sur Windows, macOS et Linux (S12)

Statut : **chantier A écrit en S12 et terminé en S13** (alerte, entrée du menu de l'icône) ; **S14 (02/11/2026) : dépôt public, vérification et installation Windows par les Releases, sans jeton** (voir la fin du fichier) ; reste la partie « installation intégrée » Mac et Linux (chantier C).

## État actuel

- **Windows seulement** : onglet « Mise à jour » (`crates/cli/src/web/update.rs`). « Chercher » interroge l'API GitHub (dernière exécution verte de la CI sur `main`), montre commit, date et taille ; après confirmation, télécharge l'artefact `scriibe-windows-x64`, remplace `scriibe.exe` (l'ancien est gardé de côté) et relance.
- La **version installée** est l'empreinte du commit (`GITHUB_SHA`), absente d'une compilation à la main (donc « jamais à jour » en local).
- **Aucune alerte** : rien ne vérifie tout seul, il faut cliquer.
- Dépôt **privé** : il faut un jeton GitHub en lecture seule (Gestionnaire d'identifiants). Les artefacts de la CI **expirent après 30 jours**.
- La CI ne publie que `scriibe.exe`. Mac et Linux : `git pull` + compilation à la main, `tools/faire_app_macos.sh`.

## Trois chantiers, indépendants

**A. Alerte de nouvelle version (tous systèmes).**
Vérification au démarrage puis toutes les 6 h : une requête légère à GitHub (aucune donnée personnelle envoyée, le jeton ne part que vers `api.github.com`). Résultat : texte « Nouvelle version disponible » sous la ligne d'état de la barre latérale, pastille sur l'onglet « Mise à jour », entrée dans le menu de l'icône. Réglage « Chercher automatiquement les mises à jour » (proposé : activé, désactivable). Sur un programme compilé à la main (pas d'empreinte), l'alerte est muette. Le jeton reste nécessaire tant que le dépôt est privé.

**B. Publication par la CI (Mac et Linux).**
- Windows : déjà fait.
- Linux : un exécutable `scriibe` (x86-64) en artefact, simple.
- macOS : une `Scriibe.app` zippée. **Coûteux** : le forfait compte macOS ×10, donc seulement à la main ou sur une étiquette de version, pas à chaque envoi. **Signature** : la signature ad hoc change à chaque compilation et fait perdre à macOS les autorisations (Surveillance de l'entrée, Micro) à chaque mise à jour. Il faut une signature stable : soit exporter le certificat « Scribe Dev » en secret GitHub (la CI signe comme le fait `tools/faire_app_macos.sh`), soit compiler et signer sur le Mac de Christophe. Sans compte Apple Developer, pas de notarisation : suffisant pour une appli installée par Scriibe lui-même (rien n'est téléchargé par un navigateur, donc pas de quarantaine) ; **pas** pour la donner à d'autres personnes.
- Option plus propre : des **versions étiquetées** (GitHub Releases, « 0.2.0 ») au lieu des artefacts : plus d'expiration, un vrai numéro de version à afficher et à comparer. Si le dépôt devient public (étape « Ouverture » de la feuille de route), les Releases se lisent **sans jeton** : l'alerte et l'installation n'ont plus rien à configurer.

**C. Installation intégrée (Mac, puis Linux).**
- Mac : télécharger le zip, vérifier la taille, remplacer `~/Applications/Scriibe.app` (ou l'emplacement actuel), relancer par `open`. Le bundle en cours d'exécution se remplace en le déplaçant de côté, comme `replace_program` le fait pour Windows.
- Linux : remplacer l'exécutable ; l'emplacement et le mode d'installation (dossier, AppImage, paquet) sont à décider après un essai sur un vrai bureau Linux, jamais fait. À reporter.

## Ordre proposé

1. **A** seul : petit, utile tout de suite, sans CI ni signature (comparaison d'empreintes, comme aujourd'hui).
2. **B-Mac + C-Mac** ensemble, quand la signature est tranchée.
3. **B-Linux + C-Linux** après l'essai sur un vrai Linux.
4. Option **versions étiquetées** : à décider avec 2.

## Décisions à prendre

1. Alerte automatique activée par défaut, ou à cocher dans les Réglages ?
2. Mac : signature par certificat exporté en secret GitHub (mise à jour sans clic mais secret à garder), ou compilation sur le Mac de Christophe ?
3. Scriibe sera-t-il donné à d'autres personnes (alors Apple Developer + notarisation, 99 $/an) ou reste-t-il pour vos propres machines ?
4. Passer aux versions étiquetées (Releases) ?

## Décisions de Christophe (01/11/2026, S13)

1. **Alerte automatique désactivée par défaut** (fait en S13 : `checkUpdates` faux par défaut). Une aide complète pour novices est écrite dans l'onglet « Mise à jour » (carte « Comment ça marche ? ») et dans la FAQ de l'Aide.
2. **Mac : compilation sur le Mac de Christophe**, pour le moment (pas de certificat exporté en secret GitHub).
3. **Donner Scriibe à d'autres personnes** : rien ne l'empêche techniquement. Sans notarisation (99 $/an), macOS affiche un avertissement au premier lancement d'une appli téléchargée par un navigateur (il faut l'ouvrir par Réglages Système → Confidentialité et sécurité → « Ouvrir quand même »), et Windows de même (SmartScreen, sans certificat Authenticode). Une mise à jour installée par Scriibe lui-même n'a pas ce problème. À reprendre si Scriibe est vraiment diffusé.
4. **Versions étiquetées (GitHub Releases) : accepté**, à écrire avec le chantier B. Scribe numérotait ses versions (`Info.plist`, préfixe de commit « Scribe x.y.z : »), mais sans étiquettes ni Releases GitHub (aucune étiquette dans son dépôt).

**Suite (S13)** : procédure d'installation pour une autre personne dans `docs/INSTALLATION_MACOS.md` ; publication d'une version Mac par `tools/publier_macos.sh` (chantier B-Mac, Release GitHub, compilation sur le Mac de Christophe) ; mise à jour décrite pas à pas dans l'onglet « Mise à jour ». Reste du chantier C (installation intégrée sur Mac) et B-Linux : non écrits. À décider : dépôt privé (inviter chaque personne) ou public (Releases lisibles sans jeton).

**S14 (02/11/2026) : Releases, sans jeton.** Le dépôt `scarou/Scriibe` est public. La version installée est le numéro de `Cargo.toml` ; la vérification (réglage « automatique » toujours décoché au départ, bouton « Chercher ») lit `https://api.github.com/repos/scarou/Scriibe/releases/latest` sans identification (60 demandes par heure et par adresse, largement assez) et compare les numéros `x.y.z`. Windows : `scriibe-windows-x64.zip` de la Release remplace `scriibe.exe` ; ce zip est compilé et ajouté par `.github/workflows/release-windows.yml` quand une Release est publiée. Mac : alerte seulement, mise à jour à la main (chantier C non fait). Publier : mettre le numéro dans `Cargo.toml`, compiler, commiter, pousser, puis `tools/publier_macos.sh x.y.z` (le script refuse si le numéro diffère de `Cargo.toml`). Le jeton, son champ, son aide et `KeyKind::GitHub` sont supprimés ; un jeton déjà rangé dans le trousseau reste inutilisé et peut être effacé à la main.
