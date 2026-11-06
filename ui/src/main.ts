import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import Overlay from './overlay/Overlay.svelte';
import { guessOs } from './lib/theme';

// La fenêtre de l'indicateur (pastille à l'écran) charge la même interface avec « #overlay » : elle ne montre que la pastille.
// La couleur de la marque dépend du système : on la pose tout de suite, sans attendre la réponse du programme (voir `guessOs`).
document.documentElement.dataset.os ??= guessOs(navigator.userAgent);
const component = location.hash === '#overlay' ? Overlay : App;
mount(component, { target: document.getElementById('app')! });
