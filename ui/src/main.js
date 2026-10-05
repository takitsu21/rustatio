import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { initializeZoom } from './lib/core/zoomStore.svelte.js';

// Apply the persisted interface zoom before the first paint
initializeZoom();

const app = mount(App, {
  target: document.getElementById('app'),
});

export default app;
