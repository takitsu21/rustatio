/**
 * Theme registry: pure data describing every theme Rustatio can render.
 *
 * Deliberately free of Svelte runes so it can be imported from plain Node
 * (`node --test src/lib/**\/*.test.js`) as well as from UI code. The store in
 * `themeStore.svelte.js` re-exports everything below.
 *
 * Each theme entry needs a matching CSS block (`:root.<id>, .<id> { ... }`)
 * defining every variable in REQUIRED_COLOR_VARS. Default themes live in
 * `ui/src/app.css`, every other family in its own file under this directory.
 * themeRegistry.test.js enforces both invariants.
 *
 * Each concrete theme also declares `pair`: its counterpart flavor in the same
 * family (a dark theme pairs with the family's light theme and vice versa).
 * Automatic mode uses those pairs to pick the light and dark themes.
 *
 * To add a theme family:
 * 1. Add its palette blocks to a new CSS file under `ui/src/lib/themes/`
 *    and import it from `ui/src/app.css`.
 * 2. Add a category below and a metadata entry per flavor to THEMES,
 *    including its `pair`.
 * 3. Run `mise run ui:test`.
 */

/** Colors every theme block must define, enforced by themeRegistry.test.js. */
export const REQUIRED_COLOR_VARS = [
  '--color-background',
  '--color-foreground',
  '--color-card',
  '--color-card-foreground',
  '--color-popover',
  '--color-popover-foreground',
  '--color-primary',
  '--color-primary-foreground',
  '--color-secondary',
  '--color-secondary-foreground',
  '--color-muted',
  '--color-muted-foreground',
  '--color-accent',
  '--color-accent-foreground',
  '--color-destructive',
  '--color-destructive-foreground',
  '--color-border',
  '--color-input',
  '--color-ring',
  '--color-stat-upload',
  '--color-stat-download',
  '--color-stat-ratio',
  '--color-stat-leecher',
  '--color-stat-danger',
];

/** Themes used by Automatic mode until the user picks their own pair. */
export const DEFAULT_LIGHT_THEME = 'light';
export const DEFAULT_DARK_THEME = 'dark';

// Available themes organized by category
export const THEME_CATEGORIES = {
  default: {
    name: 'Default',
    themes: ['system', 'light', 'dark'],
  },
  catppuccin: {
    name: 'Catppuccin',
    themes: ['latte', 'frappe', 'macchiato', 'mocha'],
  },
  'tokyo-night': {
    name: 'Tokyo Night',
    themes: ['tokyo-night', 'tokyo-storm', 'tokyo-moon', 'tokyo-light'],
  },
  dracula: {
    name: 'Dracula',
    themes: ['dracula', 'alucard'],
  },
  gruvbox: {
    name: 'Gruvbox',
    themes: ['gruvbox-dark', 'gruvbox-light'],
  },
};

// All available themes
export const THEMES = {
  // Default themes
  system: {
    id: 'system',
    name: 'Automatic',
    description: 'Follow the OS light/dark preference',
    isDark: null, // Determined by system
    category: 'default',
    icon: 'monitor',
  },
  light: {
    id: 'light',
    name: 'Light',
    description: 'Default light theme',
    isDark: false,
    category: 'default',
    icon: 'sun',
    pair: 'dark',
  },
  dark: {
    id: 'dark',
    name: 'Dark',
    description: 'Default dark theme',
    isDark: true,
    category: 'default',
    icon: 'moon',
    pair: 'light',
  },
  // Catppuccin themes
  latte: {
    id: 'latte',
    name: 'Catppuccin Latte',
    description: 'Light with warm tones',
    isDark: false,
    category: 'catppuccin',
    icon: 'sun',
    color: '#8839ef', // Mauve
    pair: 'mocha',
  },
  frappe: {
    id: 'frappe',
    name: 'Catppuccin Frappé',
    description: 'Muted dark theme',
    isDark: true,
    category: 'catppuccin',
    icon: 'coffee',
    color: '#ca9ee6', // Mauve
    pair: 'latte',
  },
  macchiato: {
    id: 'macchiato',
    name: 'Catppuccin Macchiato',
    description: 'Medium contrast dark',
    isDark: true,
    category: 'catppuccin',
    icon: 'droplet',
    color: '#c6a0f6', // Mauve
    pair: 'latte',
  },
  mocha: {
    id: 'mocha',
    name: 'Catppuccin Mocha',
    description: 'The original dark',
    isDark: true,
    category: 'catppuccin',
    icon: 'moon',
    color: '#cba6f7', // Mauve
    pair: 'latte',
  },
  // Tokyo Night themes
  'tokyo-night': {
    id: 'tokyo-night',
    name: 'Tokyo Night',
    description: 'Deep navy nights',
    isDark: true,
    category: 'tokyo-night',
    icon: 'moon-star',
    color: '#7aa2f7', // Blue
    pair: 'tokyo-light',
  },
  'tokyo-storm': {
    id: 'tokyo-storm',
    name: 'Tokyo Night Storm',
    description: 'Muted navy storm',
    isDark: true,
    category: 'tokyo-night',
    icon: 'cloud-moon',
    color: '#7dcfff', // Cyan
    pair: 'tokyo-light',
  },
  'tokyo-moon': {
    id: 'tokyo-moon',
    name: 'Tokyo Night Moon',
    description: 'Soft indigo nights',
    isDark: true,
    category: 'tokyo-night',
    icon: 'sparkles',
    color: '#82aaff', // Blue
    pair: 'tokyo-light',
  },
  'tokyo-light': {
    id: 'tokyo-light',
    name: 'Tokyo Night Light',
    description: 'Daylight blue',
    isDark: false,
    category: 'tokyo-night',
    icon: 'sun',
    color: '#2959aa', // Blue
    pair: 'tokyo-night',
  },
  // Dracula themes
  dracula: {
    id: 'dracula',
    name: 'Dracula',
    description: 'Classic vivid dark',
    isDark: true,
    category: 'dracula',
    icon: 'castle',
    color: '#bd93f9', // Purple
    pair: 'alucard',
  },
  alucard: {
    id: 'alucard',
    name: 'Alucard',
    description: 'Warm parchment light',
    isDark: false,
    category: 'dracula',
    icon: 'sun',
    color: '#644ac9', // Purple
    pair: 'dracula',
  },
  // Gruvbox themes
  'gruvbox-dark': {
    id: 'gruvbox-dark',
    name: 'Gruvbox Dark',
    description: 'Warm retro groove',
    isDark: true,
    category: 'gruvbox',
    icon: 'flame',
    color: '#fabd2f', // Yellow
    pair: 'gruvbox-light',
  },
  'gruvbox-light': {
    id: 'gruvbox-light',
    name: 'Gruvbox Light',
    description: 'Retro cream light',
    isDark: false,
    category: 'gruvbox',
    icon: 'sunrise',
    color: '#b57614', // Yellow
    pair: 'gruvbox-dark',
  },
};

/**
 * Automatic-mode counterparts of a theme: picking Catppuccin Mocha means
 * Automatic mode uses Latte for light and Mocha for dark.
 *
 * `system` and unknown ids have no counterparts and return null.
 */
export function pairPrefs(themeId) {
  const info = THEMES[themeId];
  if (!info || info.isDark === null) return null;
  if (!THEMES[info.pair]) return null;

  return info.isDark ? { light: info.pair, dark: themeId } : { light: themeId, dark: info.pair };
}

/** True when the theme exists and matches the expected light/dark tone. */
export function isValidPref(themeId, isDark) {
  const info = THEMES[themeId];
  return Boolean(info) && info.isDark === isDark;
}

/**
 * Resolve the automatic light/dark preferences for a launch: valid stored
 * values win, otherwise fall back to the saved theme's family counterparts,
 * then to the default pair.
 */
export function seedPrefs(savedTheme, storedLight, storedDark) {
  const mirror = pairPrefs(savedTheme);
  return {
    light: isValidPref(storedLight, false) ? storedLight : (mirror?.light ?? DEFAULT_LIGHT_THEME),
    dark: isValidPref(storedDark, true) ? storedDark : (mirror?.dark ?? DEFAULT_DARK_THEME),
  };
}

/**
 * Group theme ids by category, keeping category order and dropping groups
 * that end up empty (e.g. the light picker has no dark flavours to show).
 */
export function groupThemes(ids) {
  return Object.entries(THEME_CATEGORIES)
    .map(([categoryId, category]) => ({
      id: categoryId,
      name: category.name,
      ids: category.themes.filter(themeId => ids.includes(themeId)),
    }))
    .filter(group => group.ids.length > 0);
}
