/**
 * Default Preset Management
 * Stores and retrieves the user's default preset configuration.
 * When a preset is set as default, new instances will use its settings.
 */

const DEFAULT_PRESET_KEY = 'rustatio-default-preset';
export const DEFAULT_PRESET_CHANGED_EVENT = 'rustatio:default-preset-changed';

function emitDefaultPresetChanged() {
  if (
    typeof window === 'undefined' ||
    typeof window.dispatchEvent !== 'function' ||
    typeof Event !== 'function'
  ) {
    return;
  }

  window.dispatchEvent(new Event(DEFAULT_PRESET_CHANGED_EVENT));
}

/**
 * Get the currently saved default preset settings.
 * @returns {Object|null} The preset settings object, or null if none is set.
 */
export function getDefaultPreset() {
  try {
    const stored = localStorage.getItem(DEFAULT_PRESET_KEY);
    if (!stored) return null;
    return JSON.parse(stored);
  } catch {
    return null;
  }
}

/**
 * Get the ID of the currently saved default preset.
 * @returns {string|null} The preset ID, or null if none is set.
 */
export function getDefaultPresetId() {
  const preset = getDefaultPreset();
  return preset?.id ?? null;
}

/**
 * Set a preset as the default for new instances.
 * @param {Object} preset - The preset object with id, name, and settings.
 */
export function setDefaultPreset(preset) {
  if (!preset || !preset.id || !preset.settings) {
    console.error('Invalid preset object');
    return;
  }

  const data = {
    id: preset.id,
    name: preset.name,
    settings: preset.settings,
  };

  localStorage.setItem(DEFAULT_PRESET_KEY, JSON.stringify(data));
  emitDefaultPresetChanged();
}

/**
 * Clear the default preset (revert to built-in defaults).
 */
export function clearDefaultPreset() {
  localStorage.removeItem(DEFAULT_PRESET_KEY);
  emitDefaultPresetChanged();
}
