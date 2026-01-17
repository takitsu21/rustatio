<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { devLog } from '../lib/devLog.js';
  import { X } from '@lucide/svelte';

  let updateAvailable = $state(false);
  let updateVersion = $state('');
  let currentVersion = $state('');
  let checking = $state(false);
  let downloading = $state(false);
  let error = $state('');
  let installMethod = $state<'appimage' | 'deb' | 'rpm' | 'unknown'>('unknown');
  let downloadUrl = $state('');

  // Check if we're running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  async function detectInstallMethod() {
    if (!isTauri) return;

    try {
      const { platform } = await import('@tauri-apps/plugin-os');
      const currentPlatform = platform();

      // Windows and macOS use native updater (MSI, NSIS, DMG, etc.)
      if (currentPlatform === 'windows' || currentPlatform === 'macos') {
        installMethod = 'appimage'; // Treat as auto-updatable
        return;
      }

      // On Linux, check the executable path to determine install method
      const { invoke } = await import('@tauri-apps/api/core');
      const exePath = await invoke('plugin:process|current_dir').catch(() => '');

      // Check common installation paths
      if (exePath.includes('tmp/.mount_') || exePath.includes('AppImage')) {
        installMethod = 'appimage';
      } else if (exePath.includes('/usr/') || exePath.includes('/opt/')) {
        // Installed via package manager - try to detect which one
        // Default to .deb for Debian-based systems
        installMethod = 'deb';
      } else {
        installMethod = 'unknown';
      }
    } catch (e) {
      console.error('Failed to detect install method:', e);
      // Default to auto-updatable on error (safer for Windows/macOS)
      installMethod = 'appimage';
    }
  }

  async function checkForUpdates() {
    if (!isTauri) return;

    checking = true;
    error = '';

    try {
      const { check } = await import('@tauri-apps/plugin-updater');

      const update = await check();

      if (update?.available) {
        updateAvailable = true;
        updateVersion = update.version;
        currentVersion = update.currentVersion;
        devLog('log', `Update available: ${update.currentVersion} -> ${update.version}`);

        // If not AppImage, prepare download URL for manual download
        if (installMethod !== 'appimage') {
          await prepareManualDownload();
        }
      } else {
        devLog('log', 'No updates available');
      }
    } catch (e) {
      console.error('Failed to check for updates:', e);

      // Check if this is the known Linux non-AppImage error
      const errorMsg = `${e}`;
      if (
        errorMsg.includes('updater on this Linux') ||
        errorMsg.includes('invalid updater binary format')
      ) {
        // This means running as .deb/.rpm on Linux - try manual update check
        devLog('log', 'Using manual update check for package installation');
        await checkManualUpdate();
      } else {
        error = `Failed to check for updates: ${e}`;
      }
    } finally {
      checking = false;
    }
  }

  async function checkManualUpdate() {
    try {
      // Fetch the latest.json to get version info
      const response = await fetch(
        'https://github.com/takitsu21/rustatio/releases/latest/download/latest.json'
      );
      const data = await response.json();

      if (data.version) {
        const { version } = await import('@tauri-apps/api/app');
        const current = await version();

        if (data.version !== current) {
          updateAvailable = true;
          updateVersion = data.version;
          currentVersion = current;
          devLog('log', `Manual update available: ${current} -> ${data.version}`);

          await prepareManualDownload();
        }
      }
    } catch (e) {
      console.error('Failed to check for manual updates:', e);
    }
  }

  async function prepareManualDownload() {
    const { arch } = await import('@tauri-apps/plugin-os');
    const currentArch = arch();

    // Detect if system uses rpm or deb
    if (installMethod === 'unknown') {
      // Try to detect from common package manager files
      installMethod = 'deb'; // Default to deb
    }

    // Build download URL based on install method
    const archString = currentArch === 'x86_64' ? 'amd64' : currentArch;

    if (installMethod === 'deb') {
      downloadUrl = `https://github.com/takitsu21/rustatio/releases/download/v${updateVersion}/Rustatio_${updateVersion}_${archString}.deb`;
    } else if (installMethod === 'rpm') {
      downloadUrl = `https://github.com/takitsu21/rustatio/releases/download/v${updateVersion}/Rustatio-${updateVersion}-1.x86_64.rpm`;
    }
  }

  async function downloadAndInstall() {
    if (!isTauri || !updateAvailable) return;

    // For .deb/.rpm installations, open download URL instead of auto-update
    if (installMethod === 'deb' || installMethod === 'rpm') {
      if (downloadUrl) {
        const { open } = await import('@tauri-apps/plugin-shell');
        await open(downloadUrl);
      }
      return;
    }

    // For AppImage, use the built-in updater
    downloading = true;
    error = '';

    try {
      const { check } = await import('@tauri-apps/plugin-updater');
      const process = await import('@tauri-apps/plugin-process');

      const update = await check();

      if (update?.available) {
        devLog('log', 'Downloading update...');

        await update.downloadAndInstall(progress => {
          devLog('log', `Download progress: ${progress.downloaded}/${progress.total} bytes`);
        });

        devLog('log', 'Update downloaded and installed, relaunching...');
        await process.relaunch();
      }
    } catch (e) {
      console.error('Failed to download and install update:', e);
      error = `Failed to install update: ${e}`;
    } finally {
      downloading = false;
    }
  }

  function dismissUpdate() {
    updateAvailable = false;
  }

  // Check for updates on mount
  onMount(async () => {
    if (isTauri) {
      await detectInstallMethod();
      checkForUpdates();
    }
  });
</script>

{#if isTauri && updateAvailable}
  <div
    class="fixed bottom-4 right-4 bg-card border border-border rounded-lg shadow-lg p-4 max-w-sm z-50"
  >
    <div class="flex flex-col gap-3">
      <div class="flex items-start justify-between">
        <div>
          <h3 class="font-semibold text-foreground">Update Available</h3>
          <p class="text-sm text-muted-foreground mt-1">
            Version {updateVersion} is available
          </p>
          <p class="text-xs text-muted-foreground">
            Current version: {currentVersion}
          </p>
        </div>
        <button
          onclick={dismissUpdate}
          class="text-muted-foreground hover:text-foreground"
          aria-label="Dismiss"
        >
          <X size={20} />
        </button>
      </div>

      {#if error}
        <p class="text-sm text-destructive">{error}</p>
      {/if}

      <div class="flex gap-2">
        <Button onclick={downloadAndInstall} disabled={downloading} class="flex-1">
          {#if installMethod === 'deb' || installMethod === 'rpm'}
            Download (.{installMethod})
          {:else}
            {downloading ? 'Installing...' : 'Update Now'}
          {/if}
        </Button>
        <Button onclick={dismissUpdate} variant="outline">Later</Button>
      </div>

      {#if installMethod === 'deb' || installMethod === 'rpm'}
        <p class="text-xs text-muted-foreground">
          The package will be downloaded. Install it with your package manager.
        </p>
      {/if}
    </div>
  </div>
{/if}

{#if isTauri && !updateAvailable && checking}
  <div class="fixed bottom-4 right-4 bg-card border border-border rounded-lg shadow-lg p-3 z-50">
    <p class="text-sm text-muted-foreground">Checking for updates...</p>
  </div>
{/if}
