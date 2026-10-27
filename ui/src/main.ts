import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import Overlay from './overlay/Overlay.svelte';

// La fenêtre de l'indicateur (pastille à l'écran) charge la même interface avec « #overlay » : elle ne montre que la pastille.
const component = location.hash === '#overlay' ? Overlay : App;
mount(component, { target: document.getElementById('app')! });
