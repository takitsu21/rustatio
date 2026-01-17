<script>
  import { Scale, EyeOff, Server, Download, Rocket, Folder, Star } from '@lucide/svelte';

  let { icon, size = 20, class: className = '' } = $props();

  // Map icon names to Lucide components
  const iconMap = {
    scale: Scale, // balanced
    'eye-off': EyeOff, // stealth
    server: Server, // seedbox
    download: Download, // partial
    rocket: Rocket, // fast
    folder: Folder, // default/custom
    star: Star, // exported custom preset
  };

  let IconComponent = $derived(iconMap[icon] || Folder);
</script>

<IconComponent {size} class={className} />
