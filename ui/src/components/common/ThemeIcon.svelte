<script>
  import {
    Castle,
    CloudMoon,
    Coffee,
    Droplet,
    Flame,
    Monitor,
    Moon,
    MoonStar,
    Palette,
    Sparkles,
    Sun,
    Sunrise,
  } from '@lucide/svelte';
  import { THEMES } from '$lib/themes/themeStore.svelte.js';

  let { theme } = $props();

  // Icon name from the registry to Lucide component
  const ICONS = {
    monitor: Monitor,
    sun: Sun,
    moon: Moon,
    coffee: Coffee,
    droplet: Droplet,
    'moon-star': MoonStar,
    'cloud-moon': CloudMoon,
    sparkles: Sparkles,
    castle: Castle,
    flame: Flame,
    sunrise: Sunrise,
    palette: Palette,
  };

  let info = $derived(THEMES[theme]);
  let Icon = $derived(ICONS[info?.icon] ?? Palette);
</script>

<Icon size={18} style={info?.color ? `color: ${info.color}` : undefined} />
