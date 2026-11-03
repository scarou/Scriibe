# Installer Scriibe sur Linux — pas à pas

🇬🇧 [English version](INSTALLATION_LINUX.en.md)

Autres systèmes : [Windows](INSTALLATION_WINDOWS.md) · [macOS](INSTALLATION_MACOS.md)

> **État de cette procédure (S15)** : le programme Linux est **expérimental**. La touche globale, le collage et la fenêtre active ont été vérifiés sur un écran virtuel (`docs/ESSAI_LINUX.md`) ; l'essai sur un vrai bureau est en cours sur Ubuntu 26.04. Les étapes ci-dessous sont à confirmer par cet essai : si l'une ne se passe pas comme écrit, noter laquelle et la dire.

**Ce qu'il faut** : un PC Linux 64 bits (x86-64) avec une **session X11**, une connexion Internet, un micro. **Wayland n'est pas pris en charge** (voir l'étape 1).

## 1. Vérifier qu'on est en X11

Ouvrir un **terminal** (touches Ctrl+Alt+T) et taper :

```bash
echo $XDG_SESSION_TYPE
```

- **`x11`** : c'est bon, passer à l'étape 2.
- **`wayland`** : Scriibe ne peut pas voir la touche de dictée ni coller dans les autres applications. Il faut ouvrir une session X11 :
  - Sur beaucoup de distributions, à l'écran de connexion, cliquer sur la **petite roue dentée** à côté du bouton de connexion et choisir une session **Xorg** / **X11** (par exemple « Ubuntu sur Xorg »).
  - **Ubuntu 25.10 et suivantes (dont 26.04)** n'ont plus de session X11 avec le bureau GNOME. Installer un autre bureau, par exemple Xfce (qui tourne en X11) : `sudo apt update && sudo apt install -y xfce4`, puis, à l'écran de connexion, choisir **Xfce Session** avec la roue dentée. GNOME reste installé.

## 2. Télécharger Scriibe

1. Ouvrir `https://github.com/scarou/Scriibe/releases` (page publique : aucun compte nécessaire).
2. Sous la version la plus récente (numéro le plus haut), cliquer sur le fichier **`scriibe-linux-x64.zip`** dans la rubrique « Assets » (s'il n'y est pas encore, attendre quelques minutes : il est ajouté par GitHub après la publication). Il arrive dans **Téléchargements**.

## 3. Installer

Dans le terminal, une commande après l'autre :

```bash
mkdir -p ~/scriibe && unzip -o ~/Téléchargements/scriibe-linux-x64.zip -d ~/scriibe
```

```bash
chmod +x ~/scriibe/scriibe
```

(Si le dossier s'appelle `Downloads` et non `Téléchargements`, remplacer le nom dans la première commande. Si `unzip` est introuvable : `sudo apt install -y unzip`.)

Le micro demande la bibliothèque ALSA, présente sur les bureaux courants. Si le lancement se plaint de `libasound.so.2` : `sudo apt install -y libasound2t64` (ou `libasound2` sur les versions plus anciennes).

## 4. Lancer

```bash
~/scriibe/scriibe
```

Le terminal reste occupé (c'est le journal de Scriibe) et **la page de Scriibe s'ouvre dans le navigateur**. Il n'y a pas encore d'icône près de l'horloge ni de fenêtre propre à Scriibe sous Linux. Pour quitter : **Ctrl+C** dans le terminal. Pour relancer la page : ouvrir l'adresse affichée par le terminal.

## 5. Choisir comment Scriibe transcrit

Dans la page : **Réglages → Moteurs**.

**A. En ligne.** La voix part chez Deepgram, le texte chez OpenRouter pour être mis au propre. Sous Linux il n'y a pas encore de trousseau : les clés se donnent par des variables d'environnement, **avant** de lancer Scriibe, dans le même terminal :

```bash
export DEEPGRAM_API_KEY='…ta clé…'
export OPENROUTER_API_KEY='…ta clé…'
~/scriibe/scriibe
```

**B. Sur le PC (rien ne part sur Internet).** Dans **Réglages → Moteurs**, les cartes **Les programmes** et **Modèles recommandés** proposent, avec leur taille et leur source avant de demander l'accord : `whisper-server (Linux)` et `llama-server (Linux, processeur)` (compilés par GitHub pour Linux x86-64 et ajoutés à chaque Release), puis un modèle Whisper et un modèle de mise au propre. Accepter un à un, puis choisir **Whisper** et **llama.cpp**. *(Ces cartes arrivent avec la version qui suit la 0.2.2 ; avec la 0.2.2 il faut compiler `whisper-server` soi-même : voir [ESSAI_LINUX.md](ESSAI_LINUX.md).)* Les programmes se rangent dans `~/.local/share/Scriibe/engines/`. Le processeur doit connaître AVX2 (tout PC depuis 2015 environ).

## 6. Essayer

1. Ouvrir un éditeur de texte et cliquer dans une page vide.
2. **Maintenir Ctrl droit** (la touche de dictée par défaut, modifiable dans **Réglages → Dictée**), dire une phrase, **relâcher** : le texte apparaît.
3. Si rien ne se passe : **Réglages → Mesures et essais** → carte « Rapport à me coller » → « Copier le rapport », puis le transmettre à la personne qui vous a donné Scriibe.

Dans un **terminal**, le collage se fait par Ctrl+Maj+V : Scriibe envoie Ctrl+V, qui n'y marche pas ; le texte reste dans le presse-papiers.

## Installer une mise à jour

La mise à jour intégrée (**Réglages → Mise à jour**) **prévient** d'une nouvelle version mais ne l'installe pas encore sous Linux. À la main :

1. Quitter Scriibe : Ctrl+C dans le terminal.
2. Télécharger le dernier `scriibe-linux-x64.zip` comme à l'étape 2 et refaire l'étape 3 (la commande remplace l'ancien programme).
3. Relancer. Réglages, historique et dictionnaire sont conservés (ils sont rangés dans `~/.local/share/scriibe`).
