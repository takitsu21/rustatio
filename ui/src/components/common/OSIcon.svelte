<script>
  import { siApple, siMacos, siLinux, siDebian, siFedora, siUbuntu } from 'simple-icons';

  let { os, size = 16, class: className = '' } = $props();

  // Icon paths from simple-icons
  const iconPaths = {
    macos: siMacos.path,
    apple: siApple.path,
    linux: siLinux.path,
    debian: siDebian.path,
    fedora: siFedora.path,
    ubuntu: siUbuntu.path,
  };

  // Icon titles
  const iconTitles = {
    macos: siMacos.title,
    apple: siApple.title,
    linux: siLinux.title,
    debian: siDebian.title,
    fedora: siFedora.title,
    ubuntu: siUbuntu.title,
    windows: 'Windows',
  };

  // Windows icon SVG path - simple-icons doesn't have Windows
  const windowsPath =
    'M0 3.449L9.75 2.1v9.451H0m10.949-9.602L24 0v11.4H10.949M0 12.6h9.75v9.451L0 20.699M10.949 12.6H24V24l-12.9-1.801';

  const currentPath = $derived(os === 'windows' ? windowsPath : iconPaths[os]);
  const currentTitle = $derived(iconTitles[os] || os);
</script>

<svg
  role="img"
  viewBox="0 0 24 24"
  width={size}
  height={size}
  class="os-icon os-icon-{os} {className}"
  aria-label={currentTitle}
>
  <path d={currentPath} />
</svg>

<style>
  .os-icon {
    /* Default fill for unknown OS */
    fill: currentColor;
  }

  /* Windows - Blue (readable on both themes) */
  .os-icon-windows {
    fill: #0078d4;
  }

  /* macOS/Apple - Black in light, White in dark */
  .os-icon-macos,
  .os-icon-apple {
    fill: light-dark(#000000, #ffffff);
  }

  /* Linux - Yellow with dark outline for visibility */
  .os-icon-linux {
    fill: #fcc624;
  }

  /* Debian - Red (readable on both themes) */
  .os-icon-debian {
    fill: #a81d33;
  }

  /* Fedora - Blue (readable on both themes) */
  .os-icon-fedora {
    fill: #51a2da;
  }

  /* Ubuntu - Orange (readable on both themes) */
  .os-icon-ubuntu {
    fill: #e95420;
  }
</style>
