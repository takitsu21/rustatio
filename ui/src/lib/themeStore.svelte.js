/**
 * Shared theme store for consistent theming across all pages
 */

// Theme state - using module-level variables that can be imported
let theme = $state('system'); // 'system', 'light', 'dark'
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
  const names = {
    light: 'Light',
    dark: 'Dark',
    system: 'System',
  };
  return names[themeValue] || 'System';
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
    darkModeQuery.addEventListener('change', e => {
      if (theme === 'system') {
        effectiveTheme = e.matches ? 'dark' : 'light';
        if (effectiveTheme === 'dark') {
          document.documentElement.classList.add('dark');
          document.documentElement.style.colorScheme = 'dark';
        } else {
          document.documentElement.classList.remove('dark');
          document.documentElement.style.colorScheme = 'light';
        }
        document.documentElement.setAttribute('data-theme', effectiveTheme);
      }
    });
  }
}

/**
 * Apply a theme
 */
export function applyTheme(newTheme) {
  theme = newTheme;
  localStorage.setItem('rustatio-theme', newTheme);

  if (newTheme === 'system') {
    if (
      typeof window !== 'undefined' &&
      window.matchMedia &&
      window.matchMedia('(prefers-color-scheme: dark)').matches
    ) {
      effectiveTheme = 'dark';
    } else {
      effectiveTheme = 'light';
    }
  } else {
    effectiveTheme = newTheme;
  }

  // Apply dark class for Tailwind
  if (typeof document !== 'undefined') {
    if (effectiveTheme === 'dark') {
      document.documentElement.classList.add('dark');
      document.documentElement.style.colorScheme = 'dark';
    } else {
      document.documentElement.classList.remove('dark');
      document.documentElement.style.colorScheme = 'light';
    }

    // Keep data-theme for backwards compatibility
    document.documentElement.setAttribute('data-theme', effectiveTheme);
  }
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
