<script>
  import { Sun, Moon, Monitor, Coffee, Droplet, Palette } from '@lucide/svelte';

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
  <Sun size={18} />
{:else if theme === 'dark'}
  <Moon size={18} />
{:else if theme === 'system'}
  <Monitor size={18} />
{:else if theme === 'latte'}
  <Sun size={18} color={flavorColors.latte} />
{:else if theme === 'frappe'}
  <Coffee size={18} color={flavorColors.frappe} />
{:else if theme === 'macchiato'}
  <Droplet size={18} color={flavorColors.macchiato} />
{:else if theme === 'mocha'}
  <Moon size={18} color={flavorColors.mocha} />
{:else}
  <Palette size={18} />
{/if}
