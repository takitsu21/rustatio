export const ZOOM_MIN = 0.8;
export const ZOOM_MAX = 1.6;
export const ZOOM_STEP = 0.05;
export const ZOOM_DEFAULT = 1.1;
export const ZOOM_STORAGE_KEY = 'rustatio-ui-zoom';

function round2(value) {
  return Math.round(value * 100) / 100;
}

export function clampZoom(value) {
  const parsed = typeof value === 'number' ? value : parseFloat(value);
  if (!Number.isFinite(parsed)) return ZOOM_DEFAULT;
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, round2(parsed)));
}

export function stepZoom(current, direction) {
  const delta = direction === 'in' ? ZOOM_STEP : -ZOOM_STEP;
  return clampZoom(round2(clampZoom(current) + delta));
}

export function zoomPercent(value) {
  return Math.round(clampZoom(value) * 100);
}

export function parseStoredZoom(raw) {
  if (raw == null || raw === '') return ZOOM_DEFAULT;
  return clampZoom(raw);
}
