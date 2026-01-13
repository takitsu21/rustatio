<script>
  import { onMount } from 'svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { detectOS, getDownloadType } from '$lib/utils.js';

  const GITHUB_REPO = 'https://github.com/takitsu21/rustatio';

  let os = $state('unknown');
  let downloadType = $state('');
  let downloadUrl = $state('');
  let showDropdown = $state(false);
  let latestVersion = $state(null);
  let downloadOptions = $state([]);

  onMount(async () => {
    os = detectOS();
    downloadType = getDownloadType(os);

    // Fetch latest version
    latestVersion = await fetchLatestVersion();

    // Set download URL and options
    downloadUrl = await getDirectDownloadUrl(os, downloadType);
    downloadOptions = getDownloadOptions();
  });

  async function fetchLatestVersion() {
    try {
      const response = await fetch(
        'https://api.github.com/repos/takitsu21/rustatio/releases/latest'
      );
      const data = await response.json();
      return data.tag_name; // Returns version like "v0.4.0"
    } catch (error) {
      console.error('Failed to fetch latest version:', error);
      return null;
    }
  }

  async function getDirectDownloadUrl(os, type) {
    const version = await fetchLatestVersion();

    if (!version) {
      // Fallback to releases page if we can't get version
      return `${GITHUB_REPO}/releases/latest`;
    }

    // Direct download URLs for latest release
    // Pattern: Rustatio_<version>_<arch>.<ext>
    const versionNumber = version.replace('v', ''); // Remove 'v' prefix: "0.4.0"

    switch (os) {
      case 'windows':
        return `${GITHUB_REPO}/releases/download/${version}/Rustatio_${versionNumber}_x64-setup.exe`;
      case 'macos':
        return `${GITHUB_REPO}/releases/download/${version}/Rustatio_${versionNumber}_x64.dmg`;
      case 'linux':
        if (type === 'deb') {
          return `${GITHUB_REPO}/releases/download/${version}/Rustatio_${versionNumber}_amd64.deb`;
        } else if (type === 'rpm') {
          return `${GITHUB_REPO}/releases/download/${version}/Rustatio_${versionNumber}_x86_64.rpm`;
        }
        return `${GITHUB_REPO}/releases/download/${version}/Rustatio_${versionNumber}_amd64.AppImage`;
      default:
        return `${GITHUB_REPO}/releases/latest`;
    }
  }

  function getDownloadFormat(os, type) {
    if (os === 'windows') {
      return '.exe';
    } else if (os === 'macos') {
      return '.dmg';
    } else if (os === 'linux') {
      if (type === 'deb') {
        return '.deb';
      } else if (type === 'rpm') {
        return '.rpm';
      }
      return 'AppImage';
    }
    return '';
  }

  function getOSForIcon(os, downloadType) {
    // Map OS and download type to the correct icon
    if (os === 'linux') {
      if (downloadType === 'deb') {
        return 'debian';
      } else if (downloadType === 'rpm') {
        return 'fedora';
      }
      return 'linux';
    }
    return os;
  }

  function getCurrentDownloadOS() {
    // Get the OS type for the current default download
    return getOSForIcon(os, downloadType);
  }

  function toggleDropdown() {
    showDropdown = !showDropdown;
  }

  function handleDownload() {
    window.location.href = downloadUrl;
  }

  function getOSIconUrl(os) {
    // Using icons from jsdelivr CDN with devicon (developer icons)
    // These icons have proper colors and better rendering
    const baseUrl = 'https://cdn.jsdelivr.net/gh/devicons/devicon/icons';
    switch (os) {
      case 'windows':
        return `${baseUrl}/windows8/windows8-original.svg`; // Windows logo with colors
      case 'macos':
        return `${baseUrl}/apple/apple-original.svg`; // Apple logo
      case 'linux':
        return `${baseUrl}/linux/linux-original.svg`; // Tux penguin with colors
      case 'debian':
        return `${baseUrl}/debian/debian-original.svg`; // Debian red spiral
      case 'fedora':
        return `${baseUrl}/fedora/fedora-original.svg`; // Fedora blue logo
      case 'ubuntu':
        return `${baseUrl}/ubuntu/ubuntu-plain.svg`; // Ubuntu orange
      default:
        return `${baseUrl}/linux/linux-original.svg`;
    }
  }

  function getDownloadOptions() {
    if (!latestVersion) {
      return [];
    }

    const versionNumber = latestVersion.replace('v', '');

    return [
      {
        label: 'Windows (.exe)',
        os: 'windows',
        url: `${GITHUB_REPO}/releases/download/${latestVersion}/Rustatio_${versionNumber}_x64-setup.exe`,
      },
      {
        label: 'macOS (.dmg)',
        os: 'macos',
        url: `${GITHUB_REPO}/releases/download/${latestVersion}/Rustatio_${versionNumber}_x64.dmg`,
      },
      {
        label: 'Debian/Ubuntu (.deb)',
        os: 'debian',
        url: `${GITHUB_REPO}/releases/download/${latestVersion}/Rustatio_${versionNumber}_amd64.deb`,
      },
      {
        label: 'Fedora/RHEL (.rpm)',
        os: 'fedora',
        url: `${GITHUB_REPO}/releases/download/${latestVersion}/Rustatio_${versionNumber}_x86_64.rpm`,
      },
      {
        label: 'Linux (AppImage)',
        os: 'linux',
        url: `${GITHUB_REPO}/releases/download/${latestVersion}/Rustatio_${versionNumber}_amd64.AppImage`,
      },
    ];
  }
</script>

<div class="relative">
  <Button onclick={handleDownload} variant="default" size="sm">
    {#snippet children()}
      <!-- OS Icon -->
      <img
        src={getOSIconUrl(getCurrentDownloadOS())}
        alt={getCurrentDownloadOS()}
        width="16"
        height="16"
        class="inline-block"
      />
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
        <polyline points="7 10 12 15 17 10" />
        <line x1="12" y1="15" x2="12" y2="3" />
      </svg>
      <span>({getDownloadFormat(os, downloadType)})</span>
      <button
        onclick={e => {
          e.stopPropagation();
          toggleDropdown();
        }}
        class="ml-1 hover:bg-white/20 rounded px-1 transition-colors"
        aria-label="Show download options"
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <polyline points="6 9 12 15 18 9"></polyline>
        </svg>
      </button>
    {/snippet}
  </Button>

  {#if showDropdown}
    <div
      class="absolute top-[calc(100%+0.5rem)] right-0 bg-card text-card-foreground border border-border/50 rounded-xl shadow-2xl p-1.5 min-w-[220px] z-50 backdrop-blur-xl animate-in fade-in slide-in-from-top-2 duration-200"
    >
      {#each downloadOptions.filter(opt => opt.os !== getCurrentDownloadOS()) as option (option.url)}
        <a
          href={option.url}
          download
          class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all bg-transparent text-card-foreground hover:bg-secondary/80 no-underline"
          onclick={() => {
            showDropdown = false;
          }}
        >
          <img
            src={getOSIconUrl(option.os)}
            alt={option.os}
            width="16"
            height="16"
            class="inline-block flex-shrink-0 opacity-80"
          />
          <span class="flex-1 text-left text-sm font-medium">{option.label}</span>
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
            <polyline points="7 10 12 15 17 10" />
            <line x1="12" y1="15" x2="12" y2="3" />
          </svg>
        </a>
      {/each}
      <div class="border-t border-border/50 mt-1 pt-1">
        <a
          href={GITHUB_REPO}
          target="_blank"
          rel="noopener noreferrer"
          class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all bg-transparent text-card-foreground hover:bg-secondary/80 no-underline"
          onclick={() => {
            showDropdown = false;
          }}
        >
          <span class="text-lg">📦</span>
          <span class="flex-1 text-left text-sm font-medium">View on GitHub</span>
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
            <polyline points="15 3 21 3 21 9" />
            <line x1="10" y1="14" x2="21" y2="3" />
          </svg>
        </a>
      </div>
    </div>
  {/if}
</div>

<style>
  /* Close dropdown when clicking outside */
  :global(body) {
    cursor: default;
  }
</style>
