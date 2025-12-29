<script>
  import Button from '$lib/components/ui/button.svelte';
  import ThemeIcon from './ThemeIcon.svelte';

  let {
    theme,
    showThemeDropdown,
    getThemeName,
    toggleThemeDropdown,
    selectTheme,
    isRunning = false,
    isPaused = false,
    startFaking = null,
    stopFaking = null,
    pauseFaking = null,
    resumeFaking = null,
    manualUpdate = null,
  } = $props();
</script>

<header class="max-w-7xl mx-auto mb-3">
  <div class="flex justify-between items-center py-3 border-b-2 border-primary/20">
    <div class="flex flex-col">
      <h1 class="text-2xl font-bold text-foreground tracking-tight">Rustatio</h1>
      <p class="text-xs text-muted-foreground mt-0.5">Modern BitTorrent Ratio Faker</p>
    </div>
    <div class="flex items-center gap-4">
      <!-- Compact control buttons -->
      {#if startFaking && stopFaking}
        <div class="flex gap-2 items-center">
          {#if !isRunning}
            <Button
              onclick={startFaking}
              variant="default"
              size="sm"
              class="bg-gradient-to-r from-green-600 to-green-500 hover:from-green-700 hover:to-green-600 text-white shadow-lg shadow-green-500/25 border-0"
            >
              {#snippet children()}
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M8 5v14l11-7z" />
                </svg>
                <span>Start</span>
              {/snippet}
            </Button>
          {:else}
            {#if !isPaused}
              <Button
                onclick={pauseFaking}
                variant="default"
                size="sm"
                class="bg-gradient-to-r from-amber-500 to-amber-400 hover:from-amber-600 hover:to-amber-500 text-white shadow-lg shadow-amber-500/25 border-0"
              >
                {#snippet children()}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M6 4h4v16H6V4zm8 0h4v16h-4V4z" />
                  </svg>
                  <span>Pause</span>
                {/snippet}
              </Button>
            {:else}
              <Button
                onclick={resumeFaking}
                variant="default"
                size="sm"
                class="bg-gradient-to-r from-green-600 to-green-500 hover:from-green-700 hover:to-green-600 text-white shadow-lg shadow-green-500/25 border-0"
              >
                {#snippet children()}
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                    <path d="M8 5v14l11-7z" />
                  </svg>
                  <span>Resume</span>
                {/snippet}
              </Button>
            {/if}
            <Button
              onclick={manualUpdate}
              variant="outline"
              size="sm"
              class="bg-gradient-to-r from-blue-600 to-blue-500 hover:from-blue-700 hover:to-blue-600 text-white shadow-lg shadow-blue-500/25 border-0"
            >
              {#snippet children()}
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M1 4v6h6M23 20v-6h-6" />
                  <path d="M20.49 9A9 9 0 0 0 5.64 5.64L1 10m22 4l-4.64 4.36A9 9 0 0 1 3.51 15" />
                </svg>
                <span>Update</span>
              {/snippet}
            </Button>
            <Button
              onclick={stopFaking}
              variant="destructive"
              size="sm"
              class="shadow-lg shadow-red-500/25"
            >
              {#snippet children()}
                <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="6" y="6" width="12" height="12" />
                </svg>
                <span>Stop</span>
              {/snippet}
            </Button>
          {/if}
        </div>
      {/if}

      <div class="relative">
        <button
          onclick={toggleThemeDropdown}
          class="bg-secondary text-secondary-foreground border-2 border-border rounded-lg px-4 h-12 flex items-center gap-2 cursor-pointer transition-all hover:bg-primary hover:border-primary hover:text-primary-foreground active:scale-[0.98] shadow-md"
          title="Theme: {getThemeName(theme)}"
        >
          <ThemeIcon {theme} />
          <span class="text-sm font-semibold">{getThemeName(theme)}</span>
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            class="transition-transform"
          >
            <polyline points="6 9 12 15 18 9"></polyline>
          </svg>
        </button>
        {#if showThemeDropdown}
          <div
            class="absolute top-[calc(100%+0.5rem)] right-0 bg-card text-card-foreground border border-border/50 rounded-xl shadow-2xl p-1.5 min-w-[180px] z-50 backdrop-blur-xl animate-in fade-in slide-in-from-top-2 duration-200"
          >
            <button
              class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all {theme ===
              'light'
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'bg-transparent text-card-foreground hover:bg-secondary/80'}"
              onclick={() => selectTheme('light')}
            >
              <ThemeIcon theme="light" />
              <span class="flex-1 text-left text-sm font-medium">Light</span>
              {#if theme === 'light'}
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                >
                  <polyline points="20 6 9 17 4 12"></polyline>
                </svg>
              {/if}
            </button>
            <button
              class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all {theme ===
              'dark'
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'bg-transparent text-card-foreground hover:bg-secondary/80'}"
              onclick={() => selectTheme('dark')}
            >
              <ThemeIcon theme="dark" />
              <span class="flex-1 text-left text-sm font-medium">Dark</span>
              {#if theme === 'dark'}
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                >
                  <polyline points="20 6 9 17 4 12"></polyline>
                </svg>
              {/if}
            </button>
            <button
              class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all {theme ===
              'system'
                ? 'bg-primary text-primary-foreground shadow-sm'
                : 'bg-transparent text-card-foreground hover:bg-secondary/80'}"
              onclick={() => selectTheme('system')}
            >
              <ThemeIcon theme="system" />
              <span class="flex-1 text-left text-sm font-medium">System</span>
              {#if theme === 'system'}
                <svg
                  width="16"
                  height="16"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <polyline points="20 6 9 17 4 12"></polyline>
                </svg>
              {/if}
            </button>
          </div>
        {/if}
      </div>
    </div>
  </div>
</header>
