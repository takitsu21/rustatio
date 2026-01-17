<script>
  let { theme } = $props();

  // Catppuccin flavor colors (Mauve from each palette)
  const flavorColors = {
    latte: '#8839ef',
    frappe: '#ca9ee6',
    macchiato: '#c6a0f6',
    mocha: '#cba6f7',
  };
</script>

{#if theme === 'light'}
  <!-- Sun icon for Light theme -->
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <circle cx="12" cy="12" r="5"></circle>
    <line x1="12" y1="1" x2="12" y2="3"></line>
    <line x1="12" y1="21" x2="12" y2="23"></line>
    <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
    <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
    <line x1="1" y1="12" x2="3" y2="12"></line>
    <line x1="21" y1="12" x2="23" y2="12"></line>
    <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
    <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
  </svg>
{:else if theme === 'dark'}
  <!-- Moon icon for Dark theme -->
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
  </svg>
{:else if theme === 'system'}
  <!-- Monitor icon for System -->
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
    <line x1="8" y1="21" x2="16" y2="21"></line>
    <line x1="12" y1="17" x2="12" y2="21"></line>
  </svg>
{:else if theme === 'latte'}
  <!-- Sun icon for Latte (light Catppuccin) -->
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke={flavorColors.latte}
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <circle cx="12" cy="12" r="5"></circle>
    <line x1="12" y1="1" x2="12" y2="3"></line>
    <line x1="12" y1="21" x2="12" y2="23"></line>
    <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"></line>
    <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"></line>
    <line x1="1" y1="12" x2="3" y2="12"></line>
    <line x1="21" y1="12" x2="23" y2="12"></line>
    <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line>
    <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"></line>
  </svg>
{:else if theme === 'frappe'}
  <!-- Coffee cup icon for Frappé -->
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke={flavorColors.frappe}
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M17 8h1a4 4 0 1 1 0 8h-1"></path>
    <path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4V8z"></path>
    <line x1="6" y1="2" x2="6" y2="4"></line>
    <line x1="10" y1="2" x2="10" y2="4"></line>
    <line x1="14" y1="2" x2="14" y2="4"></line>
  </svg>
{:else if theme === 'macchiato'}
  <!-- Coffee bean/drop icon for Macchiato -->
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke={flavorColors.macchiato}
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M12 2c-5.33 4-8 8-8 12a8 8 0 0 0 16 0c0-4-2.67-8-8-12z"></path>
    <path d="M12 10c-2 2-3 4-3 6"></path>
  </svg>
{:else if theme === 'mocha'}
  <!-- Moon icon for Mocha (darkest Catppuccin) -->
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke={flavorColors.mocha}
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"></path>
  </svg>
{:else}
  <!-- Fallback: palette icon -->
  <svg
    width="18"
    height="18"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <circle cx="12" cy="12" r="10"></circle>
    <circle cx="12" cy="12" r="4"></circle>
    <line x1="21.17" y1="8" x2="12" y2="8"></line>
    <line x1="3.95" y1="6.06" x2="8.54" y2="14"></line>
    <line x1="10.88" y1="21.94" x2="15.46" y2="14"></line>
  </svg>
{/if}
