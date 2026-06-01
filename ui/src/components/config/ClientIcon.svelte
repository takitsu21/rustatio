<script>
  import { siBittorrent, siQbittorrent, siUtorrent, siTransmission, siDeluge } from 'simple-icons';

  let { clientId, size = 16, class: className = '' } = $props();

  const icons = {
    qbittorrent: { kind: 'simple', icon: siQbittorrent, color: `#${siQbittorrent.hex}` },
    utorrent: { kind: 'simple', icon: siUtorrent, color: `#${siUtorrent.hex}` },
    transmission: { kind: 'simple', icon: siTransmission, color: `#${siTransmission.hex}` },
    deluge: { kind: 'simple', icon: siDeluge, color: `#${siDeluge.hex}` },
    bittorrent: { kind: 'simple', icon: siBittorrent, color: `#${siBittorrent.hex}` },
    rtorrent: {
      kind: 'custom',
      title: 'rTorrent',
      color: '#8B5CF6',
      path: 'M5 3h7.5a5.5 5.5 0 0 1 0 11H9v7H5V3Zm4 4v3h3.5a1.5 1.5 0 0 0 0-3H9Zm7.5 8 2.5 6H15l-1.7-4.2H12V15h4.5Z',
    },
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
    aria-label={currentIcon.kind === 'simple' ? currentIcon.icon.title : currentIcon.title}
  >
    <path d={currentIcon.kind === 'simple' ? currentIcon.icon.path : currentIcon.path} />
  </svg>
{/if}
