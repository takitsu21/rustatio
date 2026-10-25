/**
 * Shared theme store for consistent theming across all pages
 * Supports:
 * - Original themes: Light, Dark, Automatic (system)
 * - Old Dark and Old Light: the palette and geometry of the previous UI
 * - Catppuccin flavors: Latte (light), Frappé, Macchiato, Mocha (dark)
 * - Tokyo Night flavors: Night, Storm, Moon (dark), Light
 * - Dracula (dark) and Alucard (light)
 * - Gruvbox Dark and Gruvbox Light
 *
 * Automatic mode applies two preferences: the light theme and the dark theme,
 * swapped automatically with the OS preference. Selecting a concrete theme
 * refreshes both to its family counterparts (see `pair` in themeRegistry.js).
 *
 * Theme metadata lives in themeRegistry.js (pure data, covered by unit tests);
 * this module only applies the active theme to the document.
 */

import {
  DEFAULT_DARK_THEME,
  DEFAULT_LIGHT_THEME,
  THEMES,
  THEME_CATEGORIES,
  groupThemes,
  isValidPref,
  pairPrefs,
  seedPrefs,
} from './themeRegistry.js';

// Re-export registry data so components keep a single import point
export { THEMES, THEME_CATEGORIES, groupThemes };

// Storage keys
const THEME_KEY = 'rustatio-theme';
const LIGHT_PREF_KEY = 'rustatio-theme-light';
const DARK_PREF_KEY = 'rustatio-theme-dark';

// Theme state - using module-level variables that can be imported
let theme = $state('system'); // Current theme setting
let effectiveTheme = $state('light'); // The actual applied theme
let lightTheme = $state(DEFAULT_LIGHT_THEME); // Used by system mode on light OS preference
let darkTheme = $state(DEFAULT_DARK_THEME); // Used by system mode on dark OS preference
let showThemeDropdown = $state(false);

/**
 * Get the current theme preference
 */
export function getTheme() {
  return theme;
}

/**
 * Get the theme actually applied to the document
 */
export function getEffectiveTheme() {
  return effectiveTheme;
}

/**
 * Get the theme used by system mode when the OS is in light mode
 */
export function getLightTheme() {
  return lightTheme;
}

/**
 * Get the theme used by system mode when the OS is in dark mode
 */
export function getDarkTheme() {
  return darkTheme;
}

/**
 * Get the theme dropdown visibility state
 */
export function getShowThemeDropdown() {
  return showThemeDropdown;
}

/**
 * Toggle theme dropdown visibility
 */
export function toggleThemeDropdown(event) {
  if (event) {
    event.stopPropagation();
  }
  showThemeDropdown = !showThemeDropdown;
}

/**
 * Get human-readable theme name
 */
export function getThemeName(themeValue) {
  const themeInfo = THEMES[themeValue];
  return themeInfo ? themeInfo.name : THEMES.system.name;
}

/**
 * Determine effective theme based on system preference
 */
function getSystemTheme() {
  if (typeof window !== 'undefined' && window.matchMedia) {
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }
  return 'light';
}

function storePrefs() {
  localStorage.setItem(LIGHT_PREF_KEY, lightTheme);
  localStorage.setItem(DARK_PREF_KEY, darkTheme);
}

/**
 * Resolve and apply the effective theme for the current selection
 */
function refresh() {
  effectiveTheme =
    theme === 'system' ? (getSystemTheme() === 'dark' ? darkTheme : lightTheme) : theme;
  applyThemeClasses(effectiveTheme);
}

/**
 * Initialize the theme system - should be called on app mount
 */
export function initializeTheme() {
  const savedTheme = localStorage.getItem(THEME_KEY) || 'system';
  theme = savedTheme;

  // Stored preferences win; otherwise fall back to the saved theme's family
  // counterparts, then to the default light/dark pair
  const prefs = seedPrefs(
    savedTheme,
    localStorage.getItem(LIGHT_PREF_KEY),
    localStorage.getItem(DARK_PREF_KEY)
  );
  lightTheme = prefs.light;
  darkTheme = prefs.dark;

  refresh();

  // Listen for system theme changes
  if (typeof window !== 'undefined' && window.matchMedia) {
    const darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)');
    darkModeQuery.addEventListener('change', () => {
      if (theme === 'system') {
        refresh();
      }
    });
  }
}

/**
 * Apply CSS classes for a theme
 */
function applyThemeClasses(themeName) {
  if (typeof document === 'undefined') return;

  const root = document.documentElement;

  // Remove every registered theme class, then add the active one
  root.classList.remove(...Object.keys(THEMES));
  root.classList.add(themeName);

  // color-scheme is declared per theme block in CSS, so it is already correct
  // before hydration (see the pre-paint script in ui/index.html)
  // Keep data-theme for backwards compatibility
  root.setAttribute('data-theme', themeName);
}

/**
 * Apply a theme
 */
export function applyTheme(newTheme) {
  theme = newTheme;
  localStorage.setItem(THEME_KEY, newTheme);
  refresh();
}

/**
 * Select a theme (applies it and closes dropdown)
 */
export function selectTheme(newTheme) {
  applyTheme(newTheme);

  // Selecting a concrete theme re-derives both automatic preferences
  const mirror = pairPrefs(newTheme);
  if (mirror) {
    lightTheme = mirror.light;
    darkTheme = mirror.dark;
    storePrefs();
  }

  showThemeDropdown = false;
}

/**
 * Choose the theme system mode uses for the light OS preference
 */
export function selectLightTheme(themeId) {
  // The settings UI only offers light themes; anything else is ignored
  if (!isValidPref(themeId, false)) return;

  lightTheme = themeId;
  storePrefs();
  activateAutomatic();
}

/**
 * Choose the theme system mode uses for the dark OS preference
 */
export function selectDarkTheme(themeId) {
  // The settings UI only offers dark themes; anything else is ignored
  if (!isValidPref(themeId, true)) return;

  darkTheme = themeId;
  storePrefs();
  activateAutomatic();
}

/**
 * Changing an automatic preference implies automatic mode, so the choice
 * takes effect instead of being silently ignored under a locked theme
 */
function activateAutomatic() {
  if (theme === 'system') {
    refresh();
  } else {
    applyTheme('system');
  }
}

/**
 * Handle click outside to close dropdown
 */
export function handleClickOutside(event) {
  if (showThemeDropdown) {
    const themeSelector = document.querySelector('.theme-selector');
    if (themeSelector && !themeSelector.contains(event.target)) {
      showThemeDropdown = false;
    }
  }
}
