# Installer Scriibe sur un Mac — pas à pas, pour débutants

🇬🇧 [English version](INSTALLATION_MACOS.en.md)

Autres systèmes : [Windows](INSTALLATION_WINDOWS.md) · [Linux](INSTALLATION_LINUX.md)

**Vous êtes à l'aise avec le Terminal, ou vous voulez les moteurs en ligne, la compilation ou la mise à jour à la main ?** Voir le [guide pour connaisseurs](INSTALLATION_MACOS_EXPERT.md). Ce guide-ci n'en a pas besoin.

Ce guide est écrit pour une personne qui n'a jamais installé ce genre de programme. Aucune connaissance en informatique n'est nécessaire. Faites les étapes **dans l'ordre**, une à la fois. Chaque étape se termine par une phrase « **C'est réussi quand…** » : si ce n'est pas le cas, ne passez pas à la suivante, allez à la rubrique « Si ça ne marche pas » à la fin.

**Ce qu'il faut** : un Mac à puce Apple (M1 ou plus récent), macOS 14 ou plus récent, une connexion Internet, un micro (celui du Mac suffit). **Durée** : environ 30 minutes, dont une grande partie d'attente pendant les téléchargements.

> **État de ce guide** : écrit d'après ce qui est essayé sur le Mac de son auteur. Il n'a **pas encore été suivi à la lettre par une personne débutante sur un Mac neuf** (les étapes 1 à 3 et l'installation de Homebrew à l'étape 5 en particulier). Si une étape ne se passe pas comme écrit, notez laquelle et le message affiché, et dites-le à la personne qui vous a donné Scriibe.

## Étape 0 — Vérifier votre Mac

1. Cliquez sur la pomme  en haut à gauche de l'écran, puis sur **À propos de ce Mac**.
2. Regardez la ligne **Puce** : elle doit commencer par **Apple M** (par exemple « Apple M1 »). Regardez aussi la version de **macOS** : 14 ou plus.

**C'est réussi quand** la puce est une « Apple M… » et que macOS est en version 14 ou plus. Si la puce est « Intel », Scriibe ne fonctionne pas encore sur ce Mac : arrêtez-vous là.

## Étape 1 — Télécharger Scriibe

1. Ouvrez votre navigateur (Safari, par exemple) et allez à l'adresse `https://github.com/scarou/Scriibe/releases`. C'est une page publique : vous n'avez besoin d'aucun compte.
2. Repérez la version la plus récente : c'est celle **tout en haut de la page** (le numéro le plus élevé).
3. Sous cette version, cherchez la rubrique **Assets**. Cliquez sur le fichier dont le nom finit par **`-macos.zip`** (il commence par « Scriibe »). Ne cliquez pas sur « Source code ».
4. Le fichier arrive dans le dossier **Téléchargements**.

**C'est réussi quand** le fichier `Scriibe-…-macos.zip` est dans votre dossier **Téléchargements**.

## Étape 2 — Installer Scriibe

1. Ouvrez le **Finder** (l'icône en forme de visage bleu et blanc, dans le Dock en bas de l'écran), puis cliquez sur **Téléchargements** dans la colonne de gauche.
2. Double-cliquez sur le fichier zip : une icône **Scriibe** (une plume) apparaît à côté.
3. Faites glisser cette icône **Scriibe** sur le dossier **Applications**, dans la colonne de gauche du Finder (maintenez le clic, déplacez, relâchez).

**C'est réussi quand** vous voyez **Scriibe** en ouvrant le dossier **Applications**.

## Étape 3 — Ouvrir Scriibe pour la première fois

macOS affiche un avertissement au premier lancement, parce que Scriibe n'est pas signé par Apple. C'est normal, et cela n'arrive qu'une seule fois.

1. Dans **Applications**, double-cliquez sur **Scriibe**. macOS affiche un message qui dit qu'il ne peut pas vérifier le développeur. Cliquez sur **OK** (ou **Terminé**). Scriibe ne s'ouvre pas : c'est attendu.
2. Cliquez sur la pomme , puis **Réglages Système**. Dans la colonne de gauche, cliquez sur **Confidentialité et sécurité**.
3. Faites défiler la page vers le bas jusqu'à la rubrique **Sécurité**. Une ligne dit que « Scriibe » a été bloqué. Cliquez sur **Ouvrir quand même**.
4. Si le Mac demande votre mot de passe (celui de votre session), saisissez-le. Confirmez en cliquant sur **Ouvrir**.

**C'est réussi quand** la fenêtre de Scriibe s'ouvre **et** qu'une petite icône apparaît en haut à droite de l'écran, près de l'horloge (c'est la « barre des menus »). Il n'y a pas d'icône de Scriibe dans le Dock : c'est voulu.

## Étape 4 — Donner les trois autorisations

Scriibe a besoin de trois autorisations. Pour **chacune**, faites ceci :

1. Cliquez sur la pomme , **Réglages Système**, puis **Confidentialité et sécurité**.
2. Cliquez sur la rubrique indiquée dans le tableau ci-dessous.
3. Dans la liste, **activez l'interrupteur de Scriibe**. Si Scriibe n'est pas dans la liste, il apparaît en général après la première demande de macOS ; sinon cliquez sur le **+** en bas de la liste et choisissez Scriibe dans **Applications**.
4. Cliquez sur l'icône de Scriibe dans la barre des menus, puis sur **Quitter Scriibe**. Rouvrez-le (double-clic sur Scriibe dans **Applications**). Sans cette relance, l'autorisation n'est pas prise en compte.

| Rubrique | À quoi ça sert |
|---|---|
| **Microphone** | Entendre votre voix. |
| **Surveillance de l'entrée** | Savoir quand vous maintenez la touche de dictée (la touche **Option de droite**). |
| **Contrôle de l'appareil et accès aux données** (selon la version de macOS, elle s'appelle parfois **Accessibilité**) | Coller le texte là où vous écrivez. Sans elle, le texte reste en attente : la combinaison **⌘V** le colle à la main. |

macOS peut aussi afficher lui-même une fenêtre de demande : cliquez alors sur **Autoriser**, puis quittez et rouvrez Scriibe comme ci-dessus.

**C'est réussi quand** les trois interrupteurs de Scriibe sont activés et que vous avez rouvert Scriibe après le dernier.

## Étape 5 — Installer deux petits programmes

Scriibe transcrit votre voix grâce à deux programmes qui tournent **sur votre Mac** (votre voix et votre texte ne sortent pas du Mac). Sur un Mac, ils s'installent par le **Terminal**, une fenêtre où l'on tape des commandes. Vous n'avez rien à comprendre : il suffit de **copier** chaque commande et de la **coller**.

**Comment ouvrir le Terminal** : appuyez sur les touches **⌘ (Commande) et Espace** en même temps, tapez **Terminal**, puis appuyez sur **Entrée**. Une fenêtre blanche ou noire s'ouvre.

**Comment lancer une commande** : sélectionnez la commande dans ce guide, copiez-la (**⌘C**), cliquez dans la fenêtre du Terminal, collez (**⌘V**), puis appuyez sur **Entrée**. Attendez que le Terminal « ait fini » : c'est le cas quand une nouvelle ligne se termine par le signe **%** et que le curseur attend. Ne fermez pas la fenêtre avant.

### 5a — Vérifier si Homebrew est déjà là

Homebrew est un programme qui installe les autres. Collez cette commande, puis Entrée :

```bash
brew --version
```

- Si le Terminal répond par un numéro (par exemple « Homebrew 4… »), **passez directement à l'étape 5c**.
- S'il répond « **command not found: brew** » (commande introuvable), faites l'étape 5b.

### 5b — Installer Homebrew (seulement si l'étape 5a a répondu « command not found »)

1. Ouvrez `https://brew.sh` dans le navigateur. La commande à copier est affichée en haut de la page, sous « Install Homebrew ». Elle est aussi reproduite ici :

```bash
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

2. Collez-la dans le Terminal, puis Entrée.
3. Le Terminal demande un **mot de passe** : c'est celui de votre session Mac. **Rien ne s'affiche quand vous tapez**, pas même des points : c'est normal. Tapez-le, puis Entrée.
4. Il demande ensuite d'appuyer sur **Entrée** pour continuer (« Press RETURN »). Faites-le. macOS peut proposer d'installer des « outils de développement en ligne de commande » : acceptez, et attendez la fin (plusieurs minutes).
5. À la fin, Homebrew affiche un texte avec les mots **Next steps** (« Étapes suivantes »). Faites alors ces deux commandes, l'une après l'autre (collez, Entrée, puis collez la suivante, Entrée) :

```bash
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
```

```bash
eval "$(/opt/homebrew/bin/brew shellenv)"
```

6. Refaites l'étape 5a : cette fois, `brew --version` doit répondre par un numéro.

### 5c — Installer le programme de transcription

```bash
brew install whisper-cpp
```

Attendez la fin (quelques minutes ; beaucoup de lignes défilent, c'est normal).

### 5d — Installer le programme de mise au propre

```bash
brew install llama.cpp
```

Attendez la fin, de la même façon.

**C'est réussi quand** les deux commandes sont terminées (le signe **%** est revenu) sans le mot **Error** en rouge dans les dernières lignes. Vous pouvez fermer le Terminal.

## Étape 6 — Télécharger les modèles dans Scriibe

Les « modèles » sont les fichiers qui contiennent l'intelligence de la transcription et de la mise au propre. Scriibe les télécharge pour vous et **ne télécharge rien sans votre accord** : avant chaque téléchargement, il montre le nom, la taille et la source.

1. Ouvrez Scriibe (icône de la barre des menus → **Ouvrir Scriibe**, ou la fenêtre est déjà ouverte).
2. Cliquez sur **Réglages**, puis sur l'onglet **Moteurs**. Cherchez la carte **État de l'installation** : elle dit ce qui manque.
3. Plus bas, dans la carte **Modèles recommandés**, repérez le **Modèle Whisper « base »**. Cliquez sur **Voir la taille et la source**, lisez, puis cliquez sur **Télécharger**. Attendez la fin de la barre de progression.
4. Dans la même carte, faites de même pour un modèle de mise au propre : **« Qwen 2.5 1,5 B »** (environ 1 Go) convient pour commencer.
5. **Ne tenez pas compte de la carte « Les programmes »** : elle propose surtout des programmes pour Windows et Linux, déjà remplacés ici par l'étape 5.
6. Plus haut dans le même onglet **Moteurs**, choisissez **Whisper** pour la transcription et **llama.cpp** pour la mise au propre.

**C'est réussi quand** la carte **État de l'installation** ne signale plus rien qui manque.

## Étape 7 — Essayer

1. Ouvrez **TextEdit** (cherchez-le avec **⌘ + Espace**, comme le Terminal) et choisissez **Fichier → Nouveau** : une page vide s'ouvre. Cliquez dedans pour y mettre le curseur.
2. **Maintenez la touche Option de droite** (en bas à droite du clavier, à droite de la barre d'espace). Une petite pastille apparaît : attendez qu'elle affiche « J'écoute… ».
3. Dites une phrase complète, par exemple « Bonjour, je teste Scriibe. »
4. **Relâchez** la touche. Au bout d'une ou deux secondes, le texte apparaît dans la page.

**C'est réussi quand** votre phrase s'écrit dans TextEdit.

## Si ça ne marche pas

| Ce que vous voyez | Ce qu'il faut faire |
|---|---|
| Rien ne se passe quand vous maintenez la touche | Vérifiez l'autorisation **Surveillance de l'entrée** (étape 4), puis quittez et rouvrez Scriibe. |
| La pastille s'affiche, mais le texte n'apparaît pas | Vérifiez l'autorisation **Contrôle de l'appareil et accès aux données** (étape 4). En attendant, le texte est prêt à coller : appuyez sur **⌘V**. |
| La pastille reste bloquée en attente alors que vous avez relâché la touche | Le plus souvent, appuyez de nouveau sur la touche, puis relâchez-la : la pastille se débloque. |
| Le texte est vide ou faux | Vérifiez l'autorisation **Microphone** (étape 4) et que vous avez téléchargé un modèle Whisper (étape 6). |
| Autre chose | Dans Scriibe : **Réglages → Mesures et essais**, carte **Rapport à me coller**, bouton **Copier le rapport**. Collez-le dans un message à la personne qui vous a donné Scriibe : le rapport ne contient pas le texte de vos dictées. |

## Plus tard : installer une mise à jour

Dans Scriibe : **Réglages → Mise à jour** → **Chercher une nouvelle version** → **Installer et redémarrer**, puis confirmez. Scriibe se rouvre tout seul. Réglages, historique et dictionnaire sont conservés. Si cela échoue, voir le [guide pour connaisseurs](INSTALLATION_MACOS_EXPERT.md) (mise à jour à la main).
