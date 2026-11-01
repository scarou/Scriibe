# Installer Scriibe sur un Mac — pas à pas

Pour une personne qui reçoit Scriibe d'un tiers (aucune connaissance de la programmation nécessaire). Compter 10 minutes avec les moteurs en ligne, 30 minutes avec les moteurs sur le Mac (téléchargements compris).

**Ce qu'il faut** : un Mac à puce Apple (M1 ou plus récent), macOS 14 ou plus récent, une connexion Internet, un micro.

> **Aucune version n'est encore publiée dans les « Releases »** (la première se crée avec `tools/publier_macos.sh`, voir la fin de ce fichier) : tant que la page est vide, demander le zip à la personne qui vous donne Scriibe.
>
> **État de cette procédure (S13)** : écrite d'après ce qui est essayé sur le Mac de Christophe (autorisations, première dictée, moteurs). Les étapes 1 à 3 (téléchargement d'une version publiée, avertissement de macOS au premier lancement) **n'ont pas encore été essayées sur un autre Mac** : si une étape ne se passe pas comme écrit, noter laquelle et la dire.

## 1. Télécharger Scriibe

1. Ouvrir la page des versions : `https://github.com/scarou/Scriibe/releases` (tant que le dépôt est privé, il faut un compte GitHub invité par Christophe : sans invitation, la page répond « 404 » ; une fois le dépôt public, la page est ouverte à tous).
2. Sous la version la plus récente (le numéro le plus haut, en haut de la page), cliquer sur le fichier **`Scriibe-…-macos.zip`** dans la rubrique « Assets ». Le téléchargement arrive dans le dossier **Téléchargements**.

## 2. Installer

1. Dans le Finder, ouvrir **Téléchargements** et double-cliquer sur le zip : **Scriibe** (une icône de plume) apparaît à côté.
2. Faire glisser **Scriibe** dans le dossier **Applications** de la barre latérale du Finder.

## 3. Premier lancement (avertissement de macOS)

Scriibe n'est pas signé par Apple (cela coûte 99 $ par an) : macOS le dit au premier lancement. C'est normal et sans danger pour une application dont on connaît la source.

1. Dans **Applications**, double-cliquer sur **Scriibe**. macOS affiche un message disant qu'il ne peut pas vérifier le développeur : cliquer sur **OK** (ou **Terminé**).
2. Ouvrir **Réglages Système → Confidentialité et sécurité**, descendre jusqu'à la rubrique **Sécurité** : une ligne dit que « Scriibe » a été bloqué. Cliquer sur **Ouvrir quand même**, saisir le mot de passe du Mac si macOS le demande, puis confirmer.
3. La fenêtre de Scriibe s'ouvre et une petite icône apparaît dans la **barre des menus** (en haut à droite, près de l'horloge). Il n'y a pas d'icône dans le Dock : c'est voulu.

Cet avertissement n'apparaît qu'une fois.

## 4. Donner les trois autorisations

macOS les demande quand Scriibe en a besoin. À chaque fois, aller dans **Réglages Système → Confidentialité et sécurité**, ouvrir la rubrique indiquée, **activer l'interrupteur de Scriibe**, puis **quitter Scriibe** (icône de la barre des menus → « Quitter Scriibe ») et le **relancer**.

| Rubrique | À quoi ça sert |
|---|---|
| **Microphone** | Entendre votre voix. |
| **Surveillance de l'entrée** | Savoir quand vous maintenez la touche de dictée (Option droite par défaut). |
| **Contrôle de l'appareil et accès aux données** | Coller le texte dans l'application ouverte (⌘V envoyé par Scriibe). Sans elle, le texte reste dans le presse-papiers : ⌘V le colle à la main. |

## 5. Choisir comment Scriibe transcrit

Dans Scriibe : **Réglages → Moteurs**. Deux voies, au choix (on peut changer plus tard).

**A. En ligne (le plus rapide à mettre en place).** La voix part chez Deepgram, le texte chez OpenRouter pour être mis au propre. Il faut un compte et une clé pour chacun (les deux services sont payants à l'usage, à quelques centimes la dictée) : coller chaque clé dans **Réglages → Moteurs → Clés**. Les clés sont rangées dans le trousseau du Mac et ne sont jamais affichées.

**B. Sur le Mac (rien ne part sur Internet).** Nécessite **Homebrew** (`https://brew.sh`, suivre la page). Puis, dans l'application **Terminal**, une commande après l'autre :

```bash
brew install whisper-cpp
```

```bash
brew install llama.cpp
```

Ensuite, dans Scriibe, l'onglet **Installation** dit ce qui manque (modèle Whisper, modèle de mise au propre) et propose les téléchargements, avec leur taille et leur source avant de demander l'accord. Choisir enfin **Whisper** (transcription) et **llama.cpp** (mise au propre) dans **Réglages → Moteurs**.

## 6. Essayer

1. Ouvrir **TextEdit** (ou Notes) et cliquer dans une page vide.
2. **Maintenir Option droite** (en bas à droite du clavier), dire une phrase, **relâcher** : le texte apparaît.
3. Si rien ne se passe : onglet **Journal et rapport** de Scriibe → « Copier le rapport », puis le transmettre à la personne qui vous a donné Scriibe.

## Installer une mise à jour

1. Quitter Scriibe : icône de la barre des menus → **Quitter Scriibe**.
2. Retourner sur `https://github.com/scarou/Scriibe/releases` et télécharger le zip de la **version la plus récente** (comparer son numéro et son code à ceux de l'onglet « Mise à jour » de Scriibe).
3. Double-cliquer sur le zip dans **Téléchargements**, puis faire glisser **Scriibe** dans **Applications** et cliquer sur **Remplacer**.
4. Ouvrir Scriibe. Réglages, historique et dictionnaire sont conservés (ils sont rangés à part). Les autorisations sont normalement conservées ; si macOS les redemande, les redonner comme à l'étape 4.

L'onglet **Mise à jour** de Scriibe prévient quand une version plus récente existe (voir « Alerte automatique » dans cet onglet).

---

## Pour le créateur : publier une version (Mac de Christophe)

Une version se compile et se signe sur le Mac de Christophe (certificat « Scribe Dev »), puis se publie en une commande :

```bash
tools/publier_macos.sh 0.2.0
```

Le script exige une compilation à jour (`cd ui && npm ci && npm run build`, puis `cargo build --release`), assemble `Scriibe.app`, la zippe, **montre l'aperçu** (nom, version, taille) et ne publie qu'après un « oui ». Il crée une étiquette et une publication (Release) sur `scarou/Scriibe` avec le zip en pièce jointe.
