# Installer Scriibe sur Windows — pas à pas

🇬🇧 [English version](INSTALLATION_WINDOWS.en.md)

Autres systèmes : [macOS](INSTALLATION_MACOS.md) · [Linux](INSTALLATION_LINUX.md)

Pour une personne qui reçoit Scriibe d'un tiers (aucune connaissance de la programmation nécessaire). Compter 10 minutes avec les moteurs en ligne, 30 minutes avec les moteurs sur le PC (téléchargements compris).

**Ce qu'il faut** : Windows 10 ou 11 (64 bits), une connexion Internet, un micro. Un compte GitHub gratuit n'est utile que pour la voie B de l'étape 1.

> **État de cette procédure** : écrite d'après ce qui est essayé sur le PC de Christophe (`docs/ESSAI_WINDOWS.md`). Le fichier Windows est ajouté à chaque Release par la CI quelques minutes après sa publication (la version 0.1.0, publiée avant, n'en a pas : voie B). Les étapes n'ont pas encore été essayées par une autre personne : si l'une ne se passe pas comme écrit, noter laquelle et la dire.

## 1. Télécharger Scriibe

**Voie A — version publiée (la plus simple, sans compte).**

1. Ouvrir `https://github.com/scarou/Scriibe/releases`.
2. Sous la version la plus récente (numéro le plus haut), cliquer sur le fichier **`scriibe-windows-x64.zip`** dans la rubrique « Assets » (s'il n'y est pas encore, attendre quelques minutes ou utiliser la voie B).

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

**A. Sur le PC (rien ne part sur Internet).** Dans le même onglet (**Réglages → Moteurs**), les cartes **État de l'installation**, **Les programmes** et **Modèles recommandés** disent ce qui manque (programmes `whisper-server` et `llama-server`, modèles) et propose chaque téléchargement avec sa **taille et sa source avant de demander l'accord**. Accepter un à un. Choisir ensuite **Whisper** (transcription) et **llama.cpp** (mise au propre) dans **Réglages → Moteurs**.

**B. En ligne (en dépannage).** Plus rapide sur une machine modeste, mais payant ; la voix part chez Deepgram, le texte chez OpenRouter pour être mis au propre. Il faut un compte et une clé pour chacun (services payants à l'usage, quelques centimes la dictée) : coller chaque clé dans **Réglages → Moteurs → Clés**. Les clés sont rangées dans le Gestionnaire d'identifiants de Windows et ne sont jamais affichées.

## 6. Essayer

1. Ouvrir le Bloc-notes et cliquer dans une page vide.
2. **Maintenir Ctrl droit** (la touche de dictée par défaut, modifiable dans **Réglages → Dictée**), dire une phrase, **relâcher** : le texte apparaît.
3. Si rien ne se passe : **Réglages → Mesures et essais** → carte « Rapport à me coller » → « Copier le rapport », puis le transmettre à la personne qui vous a donné Scriibe.

## Installer une mise à jour

1. Quitter Scriibe : clic droit sur l'icône près de l'horloge → **Quitter**.
2. Télécharger la version la plus récente comme à l'étape 1 et extraire le zip **dans le même dossier** (`C:\scriibe`), en acceptant de **remplacer** `scriibe.exe`.
3. Relancer `scriibe.exe`. Réglages, historique et dictionnaire sont conservés (ils sont rangés à part).

L'onglet **Réglages → Mise à jour** de Scriibe sait aussi le faire à votre place, sans compte ni jeton : **Chercher une nouvelle version** montre la dernière version publiée, sa date et la taille du téléchargement ; **Installer et redémarrer** demande confirmation, télécharge, remplace `scriibe.exe` et relance Scriibe. L'ancien programme reste à côté sous le nom `scriibe.exe.old` jusqu'au lancement suivant.
