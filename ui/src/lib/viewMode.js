const VALID_VIEW_MODES = new Set(['standard', 'grid', 'watch']);

function normalizeViewMode(value, fallback = 'standard') {
  if (!value) return fallback;
  const mode = String(value).toLowerCase();
  return VALID_VIEW_MODES.has(mode) ? mode : fallback;
}

export { normalizeViewMode };
