import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import Overlay from './overlay/Overlay.svelte';
import { guessOs } from './lib/theme';
import { i18n } from './lib/i18n.svelte';
import { uiState } from './lib/uiState';

// La fenêtre de l'indicateur (pastille à l'écran) charge la même interface avec « #overlay » : elle ne montre que la pastille.
// La couleur de la marque dépend du système : on la pose tout de suite, sans attendre la réponse du programme (voir `guessOs`).
document.documentElement.dataset.os ??= guessOs(navigator.userAgent);
const overlay = location.hash === '#overlay';
// Les états d'affichage gardés par Scriibe (cartes repliées, langue…) sont lus avant d'afficher la page ; sans réponse, valeurs par défaut.
if (!overlay) {
  await uiState.load();
  i18n.restore();
}
mount(overlay ? Overlay : App, { target: document.getElementById('app')! });
