# Installer Scriibe sur Windows — pas à pas, pour débutants

🇬🇧 [English version](INSTALLATION_WINDOWS.en.md)

Autres systèmes : [macOS](INSTALLATION_MACOS.md) · [Linux](INSTALLATION_LINUX.md)

**Vous êtes à l'aise avec les lignes de commande, ou vous voulez les moteurs en ligne, la compilation ou la mise à jour à la main ?** Voir le [guide pour connaisseurs](INSTALLATION_WINDOWS_EXPERT.md). Ce guide-ci n'en a pas besoin.

Ce guide est écrit pour une personne qui n'a jamais installé ce genre de programme. Aucune connaissance en informatique n'est nécessaire. Faites les étapes **dans l'ordre**, une à la fois. Chaque étape se termine par une phrase « **C'est réussi quand…** » : si ce n'est pas le cas, ne passez pas à la suivante, allez à la rubrique « Si ça ne marche pas » à la fin.

**Ce qu'il faut** : un PC sous Windows 10 ou 11 (64 bits), une connexion Internet, un micro. **Durée** : environ 30 minutes, dont une grande partie d'attente pendant les téléchargements. Aucun compte n'est nécessaire.

> **État de ce guide** : écrit d'après ce qui est essayé sur le PC de son auteur (`docs/ESSAI_WINDOWS.md`). Il n'a **pas encore été suivi à la lettre par une personne débutante sur un PC neuf**. Si une étape ne se passe pas comme écrit, notez laquelle et le message affiché, et dites-le à la personne qui vous a donné Scriibe.

## Étape 0 — Vérifier votre PC

1. Appuyez sur la touche **Windows** du clavier, tapez **À propos de votre PC**, puis cliquez sur le résultat.
2. Regardez la ligne **Type du système** : elle doit dire **système d'exploitation 64 bits**. Regardez aussi la ligne **Édition** : Windows 10 ou Windows 11.

**C'est réussi quand** le système est en 64 bits et que c'est Windows 10 ou 11. Sinon, Scriibe ne fonctionne pas sur ce PC : arrêtez-vous là.

## Étape 1 — Télécharger Scriibe

1. Ouvrez votre navigateur (Edge, Chrome…) et allez à l'adresse `https://github.com/scarou/Scriibe/releases`. C'est une page publique : vous n'avez besoin d'aucun compte.
2. Repérez la version la plus récente : c'est celle **tout en haut de la page** (le numéro le plus élevé).
3. Sous cette version, cherchez la rubrique **Assets** (cliquez dessus si elle est repliée). Cliquez sur le fichier **`scriibe-windows-x64.zip`**. Ne cliquez pas sur « Source code ».
4. Si le navigateur demande de confirmer le téléchargement (« Conserver »), acceptez. Le fichier arrive dans le dossier **Téléchargements**.

**C'est réussi quand** le fichier `scriibe-windows-x64.zip` est dans votre dossier **Téléchargements**. S'il manque dans la liste « Assets », attendez quelques minutes (il est ajouté peu après la publication) et rechargez la page.

## Étape 2 — Installer Scriibe

1. Ouvrez l'**Explorateur de fichiers** (l'icône de dossier jaune dans la barre des tâches), puis cliquez sur **Téléchargements** dans la colonne de gauche.
2. Faites un **clic droit** sur le fichier zip, puis **Extraire tout…**.
3. Dans la fenêtre qui s'ouvre, effacez l'adresse proposée et tapez **`C:\scriibe`**, puis cliquez sur **Extraire**.

**C'est réussi quand** le dossier `C:\scriibe` s'ouvre et contient un fichier **`scriibe.exe`**.

## Étape 3 — Ouvrir Scriibe pour la première fois

Windows affiche un avertissement au premier lancement, parce que Scriibe n'est pas signé (cela coûte cher). C'est normal, et cela n'arrive qu'une seule fois.

1. Dans `C:\scriibe`, double-cliquez sur **`scriibe.exe`**.
2. Si une fenêtre bleue **« Windows a protégé votre ordinateur »** apparaît : cliquez sur **Informations complémentaires**, puis sur **Exécuter quand même**.
3. Une fenêtre noire s'ouvre (c'est le journal : laissez-la ouverte, ne la fermez pas), puis la fenêtre de Scriibe s'ouvre.

Fermer la fenêtre de Scriibe ne quitte pas le programme : pour le quitter vraiment, faites un clic droit sur son icône près de l'horloge, puis **Quitter**. Pour la rouvrir : clic droit sur l'icône, puis **Ouvrir Scriibe**.

**C'est réussi quand** la fenêtre de Scriibe s'ouvre **et** qu'une petite icône ronde grise apparaît près de l'horloge, en bas à droite de l'écran. Si l'icône est cachée, cliquez sur la petite flèche **^** à gauche de l'horloge.

## Étape 4 — Autoriser le micro

1. Si Windows demande l'accès au micro, cliquez sur **Oui**.
2. Sinon, ou si vous avez refusé : appuyez sur la touche **Windows**, tapez **Paramètres de confidentialité du microphone**, ouvrez le résultat, et activez **Accès au microphone**, puis **Laisser les applications de bureau accéder à votre microphone**.

**C'est réussi quand** ces deux interrupteurs sont activés. Aucune autre autorisation n'est nécessaire sous Windows.

## Étape 5 — Télécharger les programmes dans Scriibe

Scriibe a besoin de deux petits programmes : l'un transcrit votre voix, l'autre met le texte au propre. Scriibe les télécharge pour vous et **ne télécharge rien sans votre accord** : avant chaque téléchargement, il montre le nom, la taille et la source. Il n'y a rien à installer à part.

1. Dans la fenêtre de Scriibe, cliquez sur **Réglages**, puis sur l'onglet **Moteurs**. La carte **État de l'installation** dit ce qui manque.
2. Plus bas, dans la carte **Les programmes**, repérez **whisper-server (Windows)** (la transcription). Cliquez sur **Voir la taille et la source**, lisez, puis cliquez sur **Télécharger**. Attendez la fin de la barre de progression : le mot **✔ installé** apparaît.
3. Faites de même pour **llama-server (Windows, processeur)** (la mise au propre). Choisissez la version « processeur » : c'est la plus simple et elle convient à tous les PC.

**C'est réussi quand** les deux programmes affichent **✔ installé**.

## Étape 6 — Télécharger les modèles dans Scriibe

Les « modèles » sont les fichiers qui contiennent l'intelligence de la transcription et de la mise au propre. Même principe : rien n'est téléchargé sans votre accord.

1. Toujours dans **Réglages → Moteurs**, descendez à la carte **Modèles recommandés**.
2. Repérez le **Modèle Whisper « base »**. Cliquez sur **Voir la taille et la source**, lisez, puis cliquez sur **Télécharger**. Attendez la fin de la barre de progression.
3. Dans la même carte, faites de même pour un modèle de mise au propre : **« Qwen 2.5 1,5 B »** (environ 1 Go) convient pour commencer.
4. Plus haut dans le même onglet **Moteurs**, choisissez **Whisper** pour la transcription et **llama.cpp** pour la mise au propre.

**C'est réussi quand** la carte **État de l'installation** ne signale plus rien qui manque (que des ✔).

## Étape 7 — Essayer

1. Ouvrez le **Bloc-notes** (touche **Windows**, tapez **Bloc-notes**, Entrée). Cliquez dans la page vide pour y mettre le curseur.
2. **Maintenez la touche Ctrl de droite** (en bas à droite du clavier, à droite de la barre d'espace). Une petite pastille apparaît : attendez qu'elle affiche « J'écoute… ».
3. Dites une phrase complète, par exemple « Bonjour, je teste Scriibe. »
4. **Relâchez** la touche. Au bout d'une ou deux secondes, le texte apparaît dans la page.

**C'est réussi quand** votre phrase s'écrit dans le Bloc-notes.

## Si ça ne marche pas

| Ce que vous voyez | Ce qu'il faut faire |
|---|---|
| La fenêtre de Scriibe ne s'ouvre pas | Clic droit sur l'icône près de l'horloge (cliquez sur la flèche **^** si elle est cachée), puis **Ouvrir Scriibe**. |
| Windows bloque `scriibe.exe` sans proposer « Exécuter quand même » | Clic droit sur `scriibe.exe` → **Propriétés** → cochez **Débloquer** en bas → **OK**, puis relancez. |
| Rien ne se passe quand vous maintenez la touche | Dictez dans une autre fenêtre que celle de Scriibe (le Bloc-notes). Vérifiez que Scriibe tourne (icône près de l'horloge) et que l'étape 4 est faite. |
| La pastille s'affiche, mais le texte n'apparaît pas | Cliquez dans la page où écrire avant de maintenir la touche. |
| La pastille reste bloquée en attente alors que vous avez relâché la touche | Appuyez de nouveau sur la touche, puis relâchez-la : la pastille se débloque. |
| Le texte est vide ou faux | Vérifiez l'étape 4 (micro) et que les étapes 5 et 6 sont toutes à ✔. |
| Un téléchargement échoue | Recommencez le même téléchargement : un échec passager de GitHub ou d'Hugging Face arrive. |
| Autre chose | Dans Scriibe : **Réglages → Mesures et essais**, carte **Rapport à me coller**, bouton **Copier le rapport**. Collez-le dans un message à la personne qui vous a donné Scriibe : le rapport ne contient pas le texte de vos dictées. |

## Plus tard : installer une mise à jour

Dans Scriibe : **Réglages → Mise à jour** → **Chercher une nouvelle version** → **Installer et redémarrer**, puis confirmez. Scriibe se rouvre tout seul. Réglages, historique et dictionnaire sont conservés. Si cela échoue, voir le [guide pour connaisseurs](INSTALLATION_WINDOWS_EXPERT.md) (mise à jour à la main).
