<script lang="ts">
  import { onMount } from 'svelte';
  import Button from '$lib/components/ui/button.svelte';

  let updateAvailable = $state(false);
  let updateVersion = $state('');
  let currentVersion = $state('');
  let checking = $state(false);
  let downloading = $state(false);
  let error = $state('');

  // Check if we're running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

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
        console.log(`Update available: ${update.currentVersion} -> ${update.version}`);
      } else {
        console.log('No updates available');
      }
    } catch (e) {
      console.error('Failed to check for updates:', e);
      error = `Failed to check for updates: ${e}`;
    } finally {
      checking = false;
    }
  }

  async function downloadAndInstall() {
    if (!isTauri || !updateAvailable) return;

    downloading = true;
    error = '';

    try {
      const { check } = await import('@tauri-apps/plugin-updater');
      const process = await import('@tauri-apps/plugin-process');

      const update = await check();

      if (update?.available) {
        console.log('Downloading update...');

        await update.downloadAndInstall(progress => {
          console.log(`Download progress: ${progress.downloaded}/${progress.total} bytes`);
        });

        console.log('Update downloaded and installed, relaunching...');
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
  onMount(() => {
    if (isTauri) {
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
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-5 w-5"
            viewBox="0 0 20 20"
            fill="currentColor"
          >
            <path
              fill-rule="evenodd"
              d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
              clip-rule="evenodd"
            />
          </svg>
        </button>
      </div>

      {#if error}
        <p class="text-sm text-destructive">{error}</p>
      {/if}

      <div class="flex gap-2">
        <Button onclick={downloadAndInstall} disabled={downloading} class="flex-1">
          {downloading ? 'Installing...' : 'Update Now'}
        </Button>
        <Button onclick={dismissUpdate} variant="outline">Later</Button>
      </div>
    </div>
  </div>
{/if}

{#if isTauri && !updateAvailable && checking}
  <div class="fixed bottom-4 right-4 bg-card border border-border rounded-lg shadow-lg p-3 z-50">
    <p class="text-sm text-muted-foreground">Checking for updates...</p>
  </div>
{/if}
