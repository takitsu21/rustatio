import { api } from '$lib/api.js';

const DEFAULT_PRESET_KEY = 'rustatio-default-preset';
export const DEFAULT_PRESET_CHANGED_EVENT = 'rustatio:default-preset-changed';

let cachedDefaultPreset = loadLocalDefaultPreset();

function loadLocalDefaultPreset() {
  if (typeof localStorage === 'undefined') {
    return null;
  }

  try {
    const stored = localStorage.getItem(DEFAULT_PRESET_KEY);
    if (!stored) return null;
    return JSON.parse(stored);
  } catch {
    return null;
  }
}

function saveLocalDefaultPreset(preset) {
  if (typeof localStorage === 'undefined') {
    return;
  }

  if (preset) {
    localStorage.setItem(DEFAULT_PRESET_KEY, JSON.stringify(preset));
  } else {
    localStorage.removeItem(DEFAULT_PRESET_KEY);
  }
}

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

export function getDefaultPreset() {
  return cachedDefaultPreset;
}

export function getDefaultPresetId() {
  return cachedDefaultPreset?.id ?? null;
}

export async function refreshDefaultPreset() {
  const next = await api.getDefaultPreset();
  const changed = JSON.stringify(cachedDefaultPreset) !== JSON.stringify(next);
  cachedDefaultPreset = next;
  saveLocalDefaultPreset(cachedDefaultPreset);
  if (changed) {
    emitDefaultPresetChanged();
  }
  return cachedDefaultPreset;
}

export async function setDefaultPreset(preset) {
  if (!preset || !preset.id || !preset.settings) {
    throw new Error('Invalid preset object');
  }

  const data = {
    id: preset.id,
    name: preset.name,
    settings: preset.settings,
  };

  await api.setDefaultPreset(data);
  cachedDefaultPreset = data;
  saveLocalDefaultPreset(data);
  emitDefaultPresetChanged();
  return data;
}

export async function clearDefaultPreset() {
  await api.clearDefaultPreset();
  cachedDefaultPreset = null;
  saveLocalDefaultPreset(null);
  emitDefaultPresetChanged();
}
