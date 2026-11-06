# Installer Scriibe sur Linux — pas à pas, pour débutants

🇬🇧 [English version](INSTALLATION_LINUX.en.md)

Autres systèmes : [Windows](INSTALLATION_WINDOWS.md) · [macOS](INSTALLATION_MACOS.md)

**Vous êtes à l'aise avec le terminal, ou vous voulez Xfce/X11, les moteurs en ligne, la compilation ou la mise à jour à la main ?** Voir le [guide pour connaisseurs](INSTALLATION_LINUX_EXPERT.md). Ce guide-ci n'en a pas besoin.

Ce guide est écrit pour une personne qui n'a jamais installé ce genre de programme. Aucune connaissance en informatique n'est nécessaire, mais il faut **copier quelques commandes dans un terminal** : c'est expliqué à chaque fois. Faites les étapes **dans l'ordre**, une à la fois. Chaque étape se termine par une phrase « **C'est réussi quand…** » : si ce n'est pas le cas, ne passez pas à la suivante, allez à la rubrique « Si ça ne marche pas » à la fin.

**Ce qu'il faut** : un PC 64 bits (x86-64) sous **Ubuntu 25.04 ou plus récent avec le bureau GNOME** (c'est le cas d'Ubuntu 26.04), une connexion Internet, un micro. **Durée** : environ 30 minutes, dont une grande partie d'attente pendant les téléchargements. Aucun compte n'est nécessaire.

> **État de ce guide** : écrit d'après ce qui est essayé sur le PC de son auteur (Ubuntu 26.04, `docs/ESSAI_LINUX.md`). Le programme Linux reste **expérimental**, et ce guide n'a **pas encore été suivi à la lettre par une personne débutante sur un PC neuf**. Si une étape ne se passe pas comme écrit, notez laquelle et le message affiché, et dites-le à la personne qui vous a donné Scriibe.

**Comment copier une commande dans le terminal** : sélectionnez la commande dans ce guide, copiez-la (Ctrl+C), cliquez dans le terminal, collez-la avec **Ctrl+Maj+V** (et non Ctrl+V), puis appuyez sur **Entrée**. Une seule commande à la fois.

## Étape 0 — Vérifier votre PC

1. Ouvrez un **terminal** : appuyez sur **Ctrl+Alt+T**.
2. Copiez-y cette commande, puis Entrée :

```bash
echo $XDG_SESSION_TYPE; uname -m
```

**C'est réussi quand** la réponse est **`wayland`** ou **`x11`**, puis **`x86_64`** (sur deux lignes). Si la seconde ligne est autre chose (par exemple `aarch64`), Scriibe ne fonctionne pas sur ce PC : arrêtez-vous là. Si vous utilisez un autre bureau que GNOME en Wayland (KDE, par exemple), voir le [guide pour connaisseurs](INSTALLATION_LINUX_EXPERT.md) (session X11).

## Étape 1 — Télécharger Scriibe

1. Ouvrez votre navigateur et allez à l'adresse `https://github.com/scarou/Scriibe/releases`. C'est une page publique : vous n'avez besoin d'aucun compte.
2. Repérez la version la plus récente : c'est celle **tout en haut de la page** (le numéro le plus élevé).
3. Sous cette version, cherchez la rubrique **Assets** (cliquez dessus si elle est repliée). Cliquez sur le fichier **`scriibe-linux-x64.zip`**. Ne cliquez pas sur « Source code ».
4. Le fichier arrive dans votre dossier **Téléchargements**.

**C'est réussi quand** le fichier `scriibe-linux-x64.zip` est dans votre dossier **Téléchargements**. S'il manque dans la liste « Assets », attendez quelques minutes (il est ajouté peu après la publication) et rechargez la page.

## Étape 2 — Installer Scriibe

Dans le terminal (**Ctrl+Alt+T** s'il est fermé), copiez ces commandes **une par une**.

1. Cette première commande crée un dossier `scriibe` chez vous et y range le programme :

```bash
mkdir -p ~/scriibe && unzip -o ~/Téléchargements/scriibe-linux-x64.zip -d ~/scriibe
```

   - Si le terminal répond que le dossier n'existe pas, votre dossier s'appelle sans doute `Downloads` : remplacez `Téléchargements` par `Downloads` dans la commande.
   - Si le terminal répond que `unzip` est introuvable, installez-le avec `sudo apt install -y unzip` (le PC demande votre mot de passe : il ne s'affiche pas quand vous le tapez, c'est normal), puis refaites la commande ci-dessus.

2. Cette seconde commande autorise le programme à se lancer :

```bash
chmod +x ~/scriibe/scriibe
```

**C'est réussi quand** la commande suivante affiche une ligne qui contient `scriibe` :

```bash
ls ~/scriibe
```

## Étape 3 — Ouvrir Scriibe pour la première fois

1. Dans le terminal, copiez :

```bash
~/scriibe/scriibe
```

2. Le terminal reste occupé : c'est le journal de Scriibe. **Laissez-le ouvert** et ne le fermez pas tant que vous utilisez Scriibe.
3. **Sous GNOME en Wayland (le cas général d'Ubuntu récent)**, GNOME ouvre des fenêtres, **une seule fois** :
   - un **raccourci clavier** pour dicter, puis un second pour dicter et traduire. GNOME n'accepte pas une touche seule comme Ctrl droit : choisissez une combinaison avec une touche ordinaire (par exemple **Ctrl + ←** pour dicter et **Ctrl + →** pour traduire, ou Ctrl + Alt + Espace), puis cliquez sur **Add** (Ajouter) ;
   - au premier collage, une demande d'autorisation de **contrôler le clavier** : c'est ce qui permet à Scriibe de coller le texte. Acceptez.
4. Si le programme se plaint d'une bibliothèque manquante, notez son nom, puis allez à « Si ça ne marche pas ».

Fermer la fenêtre de Scriibe ne quitte pas le programme : il continue d'écouter la touche. Pour le quitter vraiment : menu de l'icône dans la barre du haut → quitter, ou **Ctrl+C** dans le terminal.

**C'est réussi quand** la fenêtre de Scriibe s'ouvre **et** qu'une petite icône apparaît dans la barre du haut, près du son et de la batterie. Sans l'extension « AppIndicator » (cas de Fedora, par exemple), l'icône n'apparaît pas : la fenêtre suffit.

## Étape 4 — Le micro

Sous Ubuntu, il n'y a normalement rien à autoriser. Vérifiez seulement que votre micro est le bon : **Paramètres → Son → Entrée**, parlez, et la barre doit bouger.

**C'est réussi quand** la barre d'entrée bouge quand vous parlez.

## Étape 5 — Télécharger les programmes dans Scriibe

Scriibe a besoin de deux petits programmes : l'un transcrit votre voix, l'autre met le texte au propre. Scriibe les télécharge pour vous et **ne télécharge rien sans votre accord** : avant chaque téléchargement, il montre le nom, la taille et la source. Ils se rangent dans `~/.local/share/Scriibe/engines/`.

1. Dans la fenêtre de Scriibe, cliquez sur **Réglages**, puis sur l'onglet **Moteurs**. La carte **État de l'installation** dit ce qui manque.
2. Plus bas, dans la carte **Les programmes**, repérez **whisper-server (Linux)** (la transcription). Cliquez sur **Voir la taille et la source**, lisez, puis cliquez sur **Télécharger**. Attendez la fin de la barre de progression : le mot **✔ installé** apparaît.
3. Faites de même pour **llama-server (Linux, processeur)** (la mise au propre).

**C'est réussi quand** les deux programmes affichent **✔ installé**.

## Étape 6 — Télécharger les modèles dans Scriibe

Les « modèles » sont les fichiers qui contiennent l'intelligence de la transcription et de la mise au propre. Même principe : rien n'est téléchargé sans votre accord.

1. Toujours dans **Réglages → Moteurs**, descendez à la carte **Modèles recommandés**.
2. Repérez le **Modèle Whisper « base »**. Cliquez sur **Voir la taille et la source**, lisez, puis cliquez sur **Télécharger**. Attendez la fin de la barre de progression.
3. Dans la même carte, faites de même pour un modèle de mise au propre : **« Qwen 2.5 1,5 B »** (environ 1 Go) convient pour commencer.
4. Plus haut dans le même onglet **Moteurs**, choisissez **Whisper** pour la transcription et **llama.cpp** pour la mise au propre.

**C'est réussi quand** la carte **État de l'installation** ne signale plus rien qui manque (que des ✔).

## Étape 7 — Essayer

1. Ouvrez un éditeur de texte (par exemple **Éditeur de texte**, via la recherche des applications) et cliquez dans la page vide.
2. **Maintenez le raccourci de dictée** choisi à l'étape 3 (sous X11, c'est **Ctrl droit** par défaut). Une petite pastille apparaît en bas de l'écran : attendez qu'elle affiche « J'écoute… ».
3. Dites une phrase complète, par exemple « Bonjour, je teste Scriibe. »
4. **Relâchez** la touche. Au bout d'une ou deux secondes, le texte apparaît dans la page.

**C'est réussi quand** votre phrase s'écrit dans l'éditeur de texte.

## Si ça ne marche pas

| Ce que vous voyez | Ce qu'il faut faire |
|---|---|
| Le terminal parle de `libasound.so.2` | Copiez : `sudo apt install -y libasound2t64` (ou `libasound2` sur une version plus ancienne d'Ubuntu), puis relancez Scriibe. |
| Le terminal parle de `libwebkit2gtk-4.1.so.0` ou de `libayatana-appindicator3.so.1` | Copiez : `sudo apt install -y libwebkit2gtk-4.1-0 libayatana-appindicator3-1`, puis relancez Scriibe. |
| La fenêtre ne s'ouvre pas | Menu de l'icône dans la barre du haut → ouvrir Scriibe, ou ouvrez dans votre navigateur l'adresse affichée par le terminal. |
| Rien ne se passe quand vous maintenez le raccourci | Vérifiez que Scriibe tourne (terminal ouvert) et que le raccourci est bien celui de l'étape 3. Pour le changer : **Paramètres → Applications → Scriibe → Raccourcis clavier**, puis relancez Scriibe. |
| La pastille s'affiche, mais le texte n'apparaît pas | Cliquez dans la page où écrire avant de maintenir le raccourci, et vérifiez que vous avez accepté l'autorisation de contrôler le clavier (étape 3). |
| Dans un **terminal**, le texte n'apparaît pas | C'est normal : un terminal colle avec **Ctrl+Maj+V**. Le texte dicté reste dans le presse-papiers. |
| La pastille reste bloquée en attente alors que vous avez relâché la touche | Appuyez de nouveau sur le raccourci, puis relâchez-le : la pastille se débloque. |
| Le texte est vide ou faux | Vérifiez l'étape 4 (micro) et que les étapes 5 et 6 sont toutes à ✔. |
| Un téléchargement échoue | Recommencez le même téléchargement : un échec passager de GitHub ou d'Hugging Face arrive. |
| Autre chose | Dans Scriibe : **Réglages → Mesures et essais**, carte **Rapport à me coller**, bouton **Copier le rapport**. Collez-le dans un message à la personne qui vous a donné Scriibe : le rapport ne contient pas le texte de vos dictées. |

## Plus tard : relancer et mettre à jour

- **Relancer Scriibe** après un redémarrage du PC : ouvrez un terminal, copiez `~/scriibe/scriibe`.
- **Installer une mise à jour** : dans Scriibe, **Réglages → Mise à jour** → **Chercher une nouvelle version** → **Installer et redémarrer**, puis confirmez. Scriibe se relance tout seul. Réglages, historique et dictionnaire sont conservés. Si cela échoue, voir le [guide pour connaisseurs](INSTALLATION_LINUX_EXPERT.md) (mise à jour à la main).
