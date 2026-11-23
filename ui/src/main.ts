import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import Overlay from './overlay/Overlay.svelte';
import { guessOs } from './lib/theme';
import { restoreUiState } from './lib/boot';
import { uiState } from './lib/uiState';

// La fenêtre de l'indicateur (pastille à l'écran) charge la même interface avec « #overlay » : elle ne montre que la pastille.
// La couleur de la marque dépend du système : on la pose tout de suite, sans attendre la réponse du programme (voir `guessOs`).
document.documentElement.dataset.os ??= guessOs(navigator.userAgent);
const overlay = location.hash === '#overlay';
// Les états d'affichage gardés par Scriibe (cartes repliées, langue…) sont lus avant d'afficher la fenêtre, la pastille comprise (sa langue) ;
// sans réponse sous deux secondes, valeurs par défaut. La pastille ne les écrit jamais.
await restoreUiState();
// Un changement encore en attente (hauteur glissée…) part à la fermeture ou au rechargement de la page, pas à la prochaine visite.
addEventListener('pagehide', () => uiState.leave());
mount(overlay ? Overlay : App, { target: document.getElementById('app')! });
