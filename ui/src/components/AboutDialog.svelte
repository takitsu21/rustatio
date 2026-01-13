<script>
  import { onMount } from 'svelte';

  let { isOpen = $bindable(false) } = $props();

  /* global __APP_VERSION__ */
  let version = $state(__APP_VERSION__);
  const author = 'Dylann Batisse';
  const license = 'MIT';
  const repository = 'https://github.com/takitsu21/rustatio';

  // Check if running in Tauri
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  onMount(async () => {
    if (isTauri) {
      // In Tauri, get the native app version (may differ from web version)
      try {
        const { getVersion } = await import('@tauri-apps/api/app');
        version = await getVersion();
      } catch (e) {
        console.error('Failed to get app version:', e);
      }
    }
    // For web/server mode, __APP_VERSION__ is already set at build time
  });

  function close() {
    isOpen = false;
  }

  function handleBackdropClick(event) {
    if (event.target === event.currentTarget) {
      close();
    }
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4"
    onclick={handleBackdropClick}
    onkeydown={e => e.key === 'Escape' && close()}
    role="dialog"
    aria-modal="true"
    aria-labelledby="about-title"
    tabindex="-1"
  >
    <div
      class="bg-card text-card-foreground rounded-xl shadow-2xl max-w-md w-full p-6 border border-border animate-in fade-in zoom-in-95 duration-200"
    >
      <!-- Header -->
      <div class="flex items-start justify-between mb-4">
        <div class="flex items-center gap-3">
          <div class="w-12 h-12 bg-primary/10 rounded-lg flex items-center justify-center">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              class="text-primary"
            >
              <circle cx="12" cy="12" r="10"></circle>
              <line x1="12" y1="16" x2="12" y2="12"></line>
              <line x1="12" y1="8" x2="12.01" y2="8"></line>
            </svg>
          </div>
          <div>
            <h2 id="about-title" class="text-xl font-bold text-foreground">Rustatio</h2>
            <p class="text-sm text-muted-foreground">Modern BitTorrent Ratio Faker</p>
          </div>
        </div>
        <button
          onclick={close}
          class="p-1 rounded hover:bg-muted transition-colors"
          aria-label="Close dialog"
        >
          <svg
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
          >
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
          </svg>
        </button>
      </div>

      <!-- Content -->
      <div class="space-y-4">
        <!-- Version -->
        <div class="flex justify-between items-center py-2 border-b border-border">
          <span class="text-sm font-medium text-muted-foreground">Version</span>
          <span class="text-sm font-semibold text-foreground">{version}</span>
        </div>

        <!-- Author -->
        <div class="flex justify-between items-center py-2 border-b border-border">
          <span class="text-sm font-medium text-muted-foreground">Author</span>
          <span class="text-sm font-semibold text-foreground">{author}</span>
        </div>

        <!-- License -->
        <div class="flex justify-between items-center py-2 border-b border-border">
          <span class="text-sm font-medium text-muted-foreground">License</span>
          <span class="text-sm font-semibold text-foreground">{license}</span>
        </div>

        <!-- GitHub -->
        <div class="py-2">
          <a
            href={repository}
            target="_blank"
            rel="noopener noreferrer"
            class="flex items-center justify-center gap-2 px-4 py-2 bg-primary text-primary-foreground rounded-lg hover:bg-primary/90 transition-colors font-medium text-sm"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path
                d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"
              />
            </svg>
            <span>View on GitHub</span>
          </a>
        </div>

        <!-- Description -->
        <div class="pt-2">
          <p class="text-xs text-muted-foreground text-center leading-relaxed">
            A high-performance BitTorrent ratio faker built with Rust and Svelte. Emulates popular
            torrent clients to help you maintain optimal ratios on private trackers.
          </p>
        </div>
      </div>
    </div>
  </div>
{/if}
