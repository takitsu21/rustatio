import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import { initializeZoom } from './lib/core/zoomStore.svelte.js';
import { initRunMode } from './lib/api.js';

// Apply the persisted interface zoom before the first paint
initializeZoom();

// Detect server vs WASM before mounting so getRunMode() is final in every
// component's first render (e.g. the Settings Connection tab).
await initRunMode();

const app = mount(App, {
  target: document.getElementById('app'),
});

export default app;
