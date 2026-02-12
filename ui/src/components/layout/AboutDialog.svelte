<script>
  import { onMount } from 'svelte';
  import { Info, X, Github } from '@lucide/svelte';

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
            <Info size={24} class="text-primary" />
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
          <X size={20} />
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
            <Github size={16} />
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
