# Installer Scriibe sur Windows — pas à pas

Pour une personne qui reçoit Scriibe d'un tiers (aucune connaissance de la programmation nécessaire). Compter 10 minutes avec les moteurs en ligne, 30 minutes avec les moteurs sur le PC (téléchargements compris).

**Ce qu'il faut** : Windows 10 ou 11 (64 bits), une connexion Internet, un micro, et un compte GitHub gratuit (voir l'étape 1).

> **État de cette procédure** : écrite d'après ce qui est essayé sur le PC de Christophe (`docs/ESSAI_WINDOWS.md`). **Aucune version Windows n'est encore publiée dans les « Releases » de GitHub** : en attendant, le programme se récupère dans la CI (étape 1, voie B). Les étapes n'ont pas encore été essayées par une autre personne : si l'une ne se passe pas comme écrit, noter laquelle et la dire.

## 1. Télécharger Scriibe

**Voie A — version publiée (dès qu'il y en aura une).**

1. Ouvrir `https://github.com/scarou/Scriibe/releases`.
2. Sous la version la plus récente (numéro le plus haut), cliquer sur le fichier **`scriibe-windows-x64.zip`** dans la rubrique « Assets ».

**Voie B — dernière version compilée par GitHub (en attendant).**

1. Se connecter à GitHub (compte gratuit : `https://github.com/signup` si besoin ; GitHub ne laisse télécharger ces fichiers qu'aux personnes connectées).
2. Ouvrir `https://github.com/scarou/Scriibe/actions/workflows/ci.yml`.
3. Cliquer sur la première exécution marquée d'une **coche verte** (la plus récente), descendre jusqu'à la rubrique **Artifacts**, cliquer sur **`scriibe-windows-x64`**. Le zip arrive dans **Téléchargements**. (Ces fichiers sont effacés par GitHub après 90 jours : si la liste est vide, demander une nouvelle version.)

## 2. Installer

1. Dans l'Explorateur de fichiers, ouvrir **Téléchargements**, clic droit sur le zip → **Extraire tout…**.
2. Choisir un dossier simple, par exemple **`C:\scriibe`**, puis **Extraire**. Il contient `scriibe.exe`.

## 3. Premier lancement (avertissement de Windows)

Scriibe n'est pas signé (cela coûte cher) : Windows SmartScreen le dit au premier lancement. C'est normal pour une application dont on connaît la source.

1. Double-cliquer sur **`scriibe.exe`**.
2. Si une fenêtre bleue « Windows a protégé votre ordinateur » apparaît : cliquer sur **Informations complémentaires**, puis sur **Exécuter quand même**.
3. Une fenêtre noire (le journal) s'ouvre, une **icône ronde apparaît près de l'horloge** (grise au repos) et **la fenêtre de Scriibe s'ouvre**. Si elle ne s'ouvre pas : clic droit sur l'icône → **Ouvrir Scriibe**. Fermer la fenêtre ne quitte pas Scriibe : pour le quitter, clic droit sur l'icône → **Quitter**.

## 4. Autoriser le micro

Si Windows demande l'accès au micro, accepter. Sinon : **Paramètres → Confidentialité et sécurité → Microphone**, et activer « Laisser les applications de bureau accéder à votre microphone ».

## 5. Choisir comment Scriibe transcrit

Dans Scriibe : **Réglages → Moteurs** (au choix, modifiable plus tard).

**A. En ligne (le plus rapide).** La voix part chez Deepgram, le texte chez OpenRouter pour être mis au propre. Il faut un compte et une clé pour chacun (services payants à l'usage, quelques centimes la dictée) : coller chaque clé dans **Réglages → Moteurs → Clés**. Les clés sont rangées dans le Gestionnaire d'identifiants de Windows et ne sont jamais affichées.

**B. Sur le PC (rien ne part sur Internet).** Ouvrir l'onglet **Installation** : il dit ce qui manque (programmes `whisper-server` et `llama-server`, modèles) et propose chaque téléchargement avec sa **taille et sa source avant de demander l'accord**. Accepter un à un. Choisir ensuite **Whisper** (transcription) et **llama.cpp** (mise au propre) dans **Réglages → Moteurs**.

## 6. Essayer

1. Ouvrir le Bloc-notes et cliquer dans une page vide.
2. **Maintenir Ctrl droit** (la touche de dictée par défaut, modifiable dans **Réglages → Dictée**), dire une phrase, **relâcher** : le texte apparaît.
3. Si rien ne se passe : onglet **Journal et rapport** → « Copier le rapport », puis le transmettre à la personne qui vous a donné Scriibe.

## Installer une mise à jour

1. Quitter Scriibe : clic droit sur l'icône près de l'horloge → **Quitter**.
2. Télécharger la version la plus récente comme à l'étape 1 et extraire le zip **dans le même dossier** (`C:\scriibe`), en acceptant de **remplacer** `scriibe.exe`.
3. Relancer `scriibe.exe`. Réglages, historique et dictionnaire sont conservés (ils sont rangés à part).

L'onglet **Mise à jour** de Scriibe sait aussi chercher et installer lui-même une version plus récente, mais il demande pour l'instant un jeton GitHub en lecture seule (l'onglet explique comment le créer) : la voie manuelle ci-dessus est la plus simple pour une autre personne.
