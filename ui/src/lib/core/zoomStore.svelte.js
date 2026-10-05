import { ZOOM_DEFAULT, ZOOM_STORAGE_KEY, clampZoom, parseStoredZoom, stepZoom } from './zoom.js';

let zoom = $state(ZOOM_DEFAULT);
let isInitialized = false;

function apply(value) {
  if (typeof document === 'undefined') return;
  document.documentElement.style.setProperty('--ui-zoom', String(value));
}

function persist(value) {
  try {
    localStorage.setItem(ZOOM_STORAGE_KEY, String(value));
  } catch {
    // localStorage unavailable (private mode, tests)
  }
}

/**
 * Applies the persisted interface zoom. Call once before mounting the app.
 */
export function initializeZoom() {
  if (isInitialized) return zoom;
  isInitialized = true;

  let stored = null;
  try {
    stored = localStorage.getItem(ZOOM_STORAGE_KEY);
  } catch {
    stored = null;
  }

  zoom = parseStoredZoom(stored);
  apply(zoom);
  return zoom;
}

export function getZoom() {
  return zoom;
}

export function setZoom(value) {
  zoom = clampZoom(value);
  apply(zoom);
  persist(zoom);
  return zoom;
}

export function zoomIn() {
  return setZoom(stepZoom(zoom, 'in'));
}

export function zoomOut() {
  return setZoom(stepZoom(zoom, 'out'));
}

export function resetZoom() {
  return setZoom(1);
}
