<script>
  import { onMount } from 'svelte';
  import { getAuthToken, setAuthToken, verifyAuthToken, clearAuthToken } from '$lib/api.js';
  import Button from '$lib/components/ui/button.svelte';
  import ThemeIcon from './ThemeIcon.svelte';
  import {
    THEMES,
    THEME_CATEGORIES,
    getTheme,
    getShowThemeDropdown,
    toggleThemeDropdown,
    selectTheme,
    initializeTheme,
    handleClickOutside,
    getThemeName,
  } from '$lib/themes/themeStore.svelte.js';
  import { ChevronDown, Check, Lock, KeyRound, AlertCircle, Loader2, LogIn } from '@lucide/svelte';

  let { onAuthenticated = () => {} } = $props();

  let token = $state('');
  let rememberToken = $state(true);
  let error = $state('');
  let isVerifying = $state(false);

  // Initialize theme on mount
  onMount(() => {
    initializeTheme();

    // Add click outside listener for theme dropdown
    document.addEventListener('click', handleClickOutside);
    return () => {
      document.removeEventListener('click', handleClickOutside);
    };
  });

  // Check if there's a stored token on mount
  $effect(() => {
    const storedToken = getAuthToken();
    if (storedToken) {
      token = storedToken;
    }
  });

  async function handleSubmit(event) {
    event.preventDefault();

    if (!token.trim()) {
      error = 'Please enter an API token';
      return;
    }

    isVerifying = true;
    error = '';

    try {
      // Temporarily set the token for verification
      setAuthToken(token.trim());

      const result = await verifyAuthToken();

      if (result.valid) {
        // Token is valid
        if (!rememberToken) {
          // If not remembering, we'll keep it in memory only
          // For now, localStorage is always used for simplicity
        }
        onAuthenticated();
      } else {
        // Token is invalid - clear it
        clearAuthToken();
        error = result.error || 'Invalid token';
      }
    } catch (err) {
      clearAuthToken();
      error = err.message || 'Failed to verify token';
    } finally {
      isVerifying = false;
    }
  }
</script>

<div class="flex min-h-screen flex-col items-center justify-center bg-background p-4">
  <!-- Theme Toggle (Fixed Top-Right) -->
  <div class="fixed right-3 top-3 z-30">
    <div class="relative theme-selector">
      <button
        onclick={toggleThemeDropdown}
        class="flex h-6 items-center gap-1 px-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
        title="Theme: {getThemeName(getTheme())}"
        aria-label="Toggle theme menu"
      >
        <ThemeIcon theme={getTheme()} />
        <ChevronDown
          size={11}
          class="transition-transform {getShowThemeDropdown() ? 'rotate-180' : ''}"
        />
      </button>
      {#if getShowThemeDropdown()}
        <div
          class="absolute right-0 top-[calc(100%+0.25rem)] z-50 max-h-[26.25rem] min-w-[13.125rem] overflow-y-auto border border-border bg-popover p-1 text-popover-foreground"
        >
          {#each Object.entries(THEME_CATEGORIES) as [categoryId, category] (categoryId)}
            <!-- Category Header -->
            <div
              class="px-2 py-1 text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground {categoryId !==
              'default'
                ? 'mt-1 border-t border-border pt-1.5'
                : ''}"
            >
              {category.name}
            </div>

            {#each category.themes as themeId (themeId)}
              {@const themeOption = THEMES[themeId]}
              <button
                class="flex w-full items-center gap-2 px-2 py-1.5 text-left transition-colors cursor-pointer {getTheme() ===
                themeOption.id
                  ? 'bg-primary/15 text-foreground'
                  : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
                onclick={() => selectTheme(themeOption.id)}
              >
                <ThemeIcon theme={themeOption.id} />
                <span class="flex-1 text-xs">{themeOption.name}</span>
                {#if getTheme() === themeOption.id}
                  <Check size={12} strokeWidth={2.5} />
                {/if}
              </button>
            {/each}
          {/each}
        </div>
      {/if}
    </div>
  </div>

  <div class="relative w-full max-w-md">
    <!-- Logo and Title -->
    <div class="mb-5 text-center">
      <!-- Logo Icon -->
      <div class="mb-3 inline-flex items-center justify-center">
        <img
          src="/android-chrome-512x512.png"
          alt="Rustatio"
          width="56"
          height="56"
          class="object-contain"
        />
      </div>

      <h1 class="text-lg font-bold uppercase tracking-[0.18em] text-foreground">Rustatio</h1>
      <p class="mt-1 text-[0.625rem] uppercase tracking-wider text-muted-foreground">
        BitTorrent ratio faker · self-hosted
      </p>
    </div>

    <!-- Auth Card -->
    <div class="overflow-hidden border border-border bg-card text-card-foreground">
      <!-- Card Header -->
      <div class="flex items-center gap-2.5 border-b border-border px-4 py-3">
        <span
          class="flex h-7 w-7 items-center justify-center border border-stat-ratio/40 bg-stat-ratio/10"
        >
          <Lock size={13} class="text-stat-ratio" />
        </span>
        <div>
          <h2 class="text-xs font-semibold uppercase tracking-wider text-foreground">
            Authentication required
          </h2>
          <p class="text-[0.625rem] text-muted-foreground">Enter your API token to continue</p>
        </div>
      </div>

      <!-- Card Body -->
      <form onsubmit={handleSubmit} class="space-y-4 p-4">
        <div>
          <label
            for="api-token"
            class="mb-1 block text-[0.625rem] uppercase tracking-wider text-muted-foreground"
          >
            API token
          </label>
          <div class="relative">
            <div class="pointer-events-none absolute inset-y-0 left-0 flex items-center pl-2.5">
              <KeyRound size={13} class="text-muted-foreground" />
            </div>
            <input
              id="api-token"
              type="password"
              bind:value={token}
              placeholder="Enter your API token"
              autocomplete="current-password"
              class="h-9 w-full border border-input bg-background pl-8 pr-2.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
              disabled={isVerifying}
            />
          </div>
          <p class="mt-1.5 text-[0.625rem] text-muted-foreground">
            This is the <code class="border border-border bg-muted px-1 py-px text-foreground"
              >AUTH_TOKEN</code
            > environment variable set on the server.
          </p>
        </div>

        <div class="flex items-center gap-2">
          <input
            id="remember-token"
            type="checkbox"
            bind:checked={rememberToken}
            class="h-3.5 w-3.5 cursor-pointer rounded-none border-input accent-primary"
            disabled={isVerifying}
          />
          <label
            for="remember-token"
            class="cursor-pointer select-none text-xs text-muted-foreground"
          >
            Remember this token
          </label>
        </div>

        {#if error}
          <div
            class="flex items-start gap-2.5 border border-stat-leecher/40 bg-stat-leecher/10 p-2.5"
          >
            <AlertCircle size={14} class="mt-0.5 flex-shrink-0 text-stat-leecher" />
            <p class="text-[0.6875rem] text-stat-leecher">{error}</p>
          </div>
        {/if}

        <Button type="submit" class="w-full" disabled={isVerifying}>
          {#if isVerifying}
            <Loader2 size={13} class="animate-spin" />
            Verifying…
          {:else}
            <LogIn size={13} />
            Connect
          {/if}
        </Button>
      </form>
    </div>

    <!-- Footer -->
    <div class="mt-5 text-center">
      <p class="text-[0.625rem] text-muted-foreground">Running in self-hosted server mode</p>
    </div>
  </div>
</div>
