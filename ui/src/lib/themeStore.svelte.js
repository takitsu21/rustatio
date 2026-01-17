/**
 * Shared theme store for consistent theming across all pages
 * Supports:
 * - Original themes: Light, Dark, System
 * - Catppuccin flavors: Latte (light), Frappé, Macchiato, Mocha (dark)
 */

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
};

// All available themes
export const THEMES = {
  // Default themes
  system: {
    id: 'system',
    name: 'System',
    description: 'Follow system preference',
    isDark: null, // Determined by system
    category: 'default',
  },
  light: {
    id: 'light',
    name: 'Light',
    description: 'Default light theme',
    isDark: false,
    category: 'default',
  },
  dark: {
    id: 'dark',
    name: 'Dark',
    description: 'Default dark theme',
    isDark: true,
    category: 'default',
  },
  // Catppuccin themes
  latte: {
    id: 'latte',
    name: 'Catppuccin Latte',
    description: 'Light with warm tones',
    isDark: false,
    category: 'catppuccin',
    color: '#8839ef', // Mauve
  },
  frappe: {
    id: 'frappe',
    name: 'Catppuccin Frappé',
    description: 'Muted dark theme',
    isDark: true,
    category: 'catppuccin',
    color: '#ca9ee6', // Mauve
  },
  macchiato: {
    id: 'macchiato',
    name: 'Catppuccin Macchiato',
    description: 'Medium contrast dark',
    isDark: true,
    category: 'catppuccin',
    color: '#c6a0f6', // Mauve
  },
  mocha: {
    id: 'mocha',
    name: 'Catppuccin Mocha',
    description: 'The original dark',
    isDark: true,
    category: 'catppuccin',
    color: '#cba6f7', // Mauve
  },
};

// Theme state - using module-level variables that can be imported
let theme = $state('system'); // Current theme setting
let effectiveTheme = $state('light'); // The actual applied theme
let showThemeDropdown = $state(false);

/**
 * Get the current theme preference
 */
export function getTheme() {
  return theme;
}

/**
 * Get the effective theme (what's actually applied)
 */
export function getEffectiveTheme() {
  return effectiveTheme;
}

/**
 * Check if the current effective theme is dark
 */
export function isDarkTheme() {
  const themeInfo = THEMES[effectiveTheme];
  if (themeInfo && themeInfo.isDark !== null) {
    return themeInfo.isDark;
  }
  // For system theme, check the effective theme
  if (effectiveTheme === 'dark') return true;
  if (effectiveTheme === 'light') return false;
  // Check actual applied class
  if (typeof document !== 'undefined') {
    return document.documentElement.classList.contains('dark');
  }
  return false;
}

/**
 * Get the theme dropdown visibility state
 */
export function getShowThemeDropdown() {
  return showThemeDropdown;
}

/**
 * Set the theme dropdown visibility
 */
export function setShowThemeDropdown(value) {
  showThemeDropdown = value;
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
  return themeInfo ? themeInfo.name : 'System';
}

/**
 * Get theme description
 */
export function getThemeDescription(themeValue) {
  const themeInfo = THEMES[themeValue];
  return themeInfo ? themeInfo.description : '';
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

/**
 * Initialize the theme system - should be called on app mount
 */
export function initializeTheme() {
  const savedTheme = localStorage.getItem('rustatio-theme') || 'system';
  theme = savedTheme;
  applyTheme(savedTheme);

  // Listen for system theme changes
  if (typeof window !== 'undefined' && window.matchMedia) {
    const darkModeQuery = window.matchMedia('(prefers-color-scheme: dark)');
    darkModeQuery.addEventListener('change', () => {
      if (theme === 'system') {
        const newEffective = getSystemTheme();
        effectiveTheme = newEffective;
        applyThemeClasses(newEffective);
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

  // Remove all theme classes
  root.classList.remove('light', 'dark', 'latte', 'frappe', 'macchiato', 'mocha');

  // Add the appropriate class
  root.classList.add(themeName);

  // Set color scheme for native elements
  const themeInfo = THEMES[themeName];
  if (themeInfo && themeInfo.isDark !== null) {
    root.style.colorScheme = themeInfo.isDark ? 'dark' : 'light';
  } else {
    // For system or unknown, determine from name
    root.style.colorScheme = themeName === 'dark' ? 'dark' : 'light';
  }

  // Keep data-theme for backwards compatibility
  root.setAttribute('data-theme', themeName);
}

/**
 * Apply a theme
 */
export function applyTheme(newTheme) {
  theme = newTheme;
  localStorage.setItem('rustatio-theme', newTheme);

  if (newTheme === 'system') {
    effectiveTheme = getSystemTheme();
  } else {
    effectiveTheme = newTheme;
  }

  applyThemeClasses(effectiveTheme);
}

/**
 * Select a theme (applies it and closes dropdown)
 */
export function selectTheme(newTheme) {
  applyTheme(newTheme);
  showThemeDropdown = false;
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
