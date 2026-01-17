<script>
  import { siQbittorrent, siUtorrent, siTransmission, siDeluge } from 'simple-icons';

  let { clientId, size = 16, class: className = '' } = $props();

  const icons = {
    qbittorrent: { icon: siQbittorrent, color: `#${siQbittorrent.hex}` },
    utorrent: { icon: siUtorrent, color: `#${siUtorrent.hex}` },
    transmission: { icon: siTransmission, color: `#${siTransmission.hex}` },
    deluge: { icon: siDeluge, color: `#${siDeluge.hex}` },
  };

  const currentIcon = $derived(icons[clientId]);
</script>

{#if currentIcon}
  <svg
    role="img"
    viewBox="0 0 24 24"
    width={size}
    height={size}
    fill={currentIcon.color}
    class={className}
    aria-label={currentIcon.icon.title}
  >
    <path d={currentIcon.icon.path} />
  </svg>
{/if}
