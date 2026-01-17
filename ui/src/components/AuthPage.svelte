<script>
  import { onMount } from 'svelte';
  import { getAuthToken, setAuthToken, verifyAuthToken, clearAuthToken } from '../lib/api.js';
  import Button from '$lib/components/ui/button.svelte';
  import ThemeIcon from './ThemeIcon.svelte';
  import {
    getTheme,
    getShowThemeDropdown,
    toggleThemeDropdown,
    selectTheme,
    initializeTheme,
    handleClickOutside,
    getThemeName,
  } from '../lib/themeStore.svelte.js';

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

<div class="min-h-screen bg-background flex flex-col items-center justify-center p-4">
  <!-- Theme Toggle (Fixed Top-Right) -->
  <div class="fixed top-4 right-4 z-30">
    <div class="relative theme-selector">
      <button
        onclick={toggleThemeDropdown}
        class="bg-secondary text-secondary-foreground border-2 border-border rounded-lg p-2 flex items-center gap-2 cursor-pointer transition-all hover:bg-primary hover:border-primary hover:text-primary-foreground active:scale-[0.98] shadow-lg"
        title="Theme: {getThemeName(getTheme())}"
        aria-label="Toggle theme menu"
      >
        <ThemeIcon theme={getTheme()} />
        <svg
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          class="transition-transform {getShowThemeDropdown() ? 'rotate-180' : ''}"
        >
          <polyline points="6 9 12 15 18 9"></polyline>
        </svg>
      </button>
      {#if getShowThemeDropdown()}
        <div
          class="absolute top-[calc(100%+0.5rem)] right-0 bg-card text-card-foreground border border-border/50 rounded-xl shadow-2xl p-1.5 min-w-[180px] z-50 backdrop-blur-xl animate-in fade-in slide-in-from-top-2 duration-200"
        >
          <button
            class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all {getTheme() ===
            'light'
              ? 'bg-primary text-primary-foreground shadow-sm'
              : 'bg-transparent text-card-foreground hover:bg-secondary/80'}"
            onclick={() => selectTheme('light')}
          >
            <ThemeIcon theme="light" />
            <span class="flex-1 text-left text-sm font-medium">Light</span>
            {#if getTheme() === 'light'}
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
            class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all {getTheme() ===
            'dark'
              ? 'bg-primary text-primary-foreground shadow-sm'
              : 'bg-transparent text-card-foreground hover:bg-secondary/80'}"
            onclick={() => selectTheme('dark')}
          >
            <ThemeIcon theme="dark" />
            <span class="flex-1 text-left text-sm font-medium">Dark</span>
            {#if getTheme() === 'dark'}
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
            class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all {getTheme() ===
            'system'
              ? 'bg-primary text-primary-foreground shadow-sm'
              : 'bg-transparent text-card-foreground hover:bg-secondary/80'}"
            onclick={() => selectTheme('system')}
          >
            <ThemeIcon theme="system" />
            <span class="flex-1 text-left text-sm font-medium">System</span>
            {#if getTheme() === 'system'}
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

  <!-- Background gradient decoration -->
  <div class="absolute inset-0 overflow-hidden pointer-events-none">
    <div class="absolute -top-40 -right-40 w-80 h-80 bg-primary/10 rounded-full blur-3xl"></div>
    <div class="absolute -bottom-40 -left-40 w-80 h-80 bg-primary/5 rounded-full blur-3xl"></div>
  </div>

  <div class="relative w-full max-w-md">
    <!-- Logo and Title -->
    <div class="text-center mb-8">
      <!-- Logo Icon -->
      <div class="inline-flex items-center justify-center mb-6">
        <img
          src="/android-chrome-512x512.png"
          alt="Rustatio"
          width="96"
          height="96"
          class="object-contain"
        />
      </div>

      <h1 class="text-3xl font-bold text-foreground tracking-tight mb-2">Rustatio</h1>
      <p class="text-muted-foreground">Modern BitTorrent Ratio Faker</p>
    </div>

    <!-- Auth Card -->
    <div
      class="bg-card text-card-foreground rounded-2xl shadow-2xl border border-border/50 overflow-hidden"
    >
      <!-- Card Header -->
      <div class="px-8 pt-8 pb-4">
        <div class="flex items-center gap-3 mb-2">
          <div class="w-10 h-10 bg-amber-500/10 rounded-xl flex items-center justify-center">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              class="text-amber-500"
            >
              <rect x="3" y="11" width="18" height="11" rx="2" ry="2"></rect>
              <path d="M7 11V7a5 5 0 0 1 10 0v4"></path>
            </svg>
          </div>
          <div>
            <h2 class="text-lg font-semibold text-foreground">Authentication Required</h2>
            <p class="text-sm text-muted-foreground">Enter your API token to continue</p>
          </div>
        </div>
      </div>

      <!-- Card Body -->
      <form onsubmit={handleSubmit} class="px-8 pb-8 space-y-5">
        <div>
          <label for="api-token" class="block text-sm font-medium text-foreground mb-2">
            API Token
          </label>
          <div class="relative">
            <div class="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
              <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                class="text-muted-foreground"
              >
                <path
                  d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"
                ></path>
              </svg>
            </div>
            <input
              id="api-token"
              type="password"
              bind:value={token}
              placeholder="Enter your API token"
              autocomplete="current-password"
              class="w-full pl-10 pr-4 py-3 text-sm border border-border rounded-xl bg-background focus:outline-none focus:ring-2 focus:ring-primary/50 focus:border-primary transition-all"
              disabled={isVerifying}
            />
          </div>
          <p class="mt-2 text-xs text-muted-foreground">
            This is the <code class="px-1.5 py-0.5 bg-muted rounded text-foreground"
              >AUTH_TOKEN</code
            > environment variable set on the server.
          </p>
        </div>

        <div class="flex items-center gap-2">
          <input
            id="remember-token"
            type="checkbox"
            bind:checked={rememberToken}
            class="w-4 h-4 rounded border-border text-primary focus:ring-primary/50 cursor-pointer"
            disabled={isVerifying}
          />
          <label
            for="remember-token"
            class="text-sm text-muted-foreground cursor-pointer select-none"
          >
            Remember this token
          </label>
        </div>

        {#if error}
          <div class="p-4 rounded-xl bg-red-500/10 border border-red-500/20 flex items-start gap-3">
            <svg
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              class="text-red-500 flex-shrink-0 mt-0.5"
            >
              <circle cx="12" cy="12" r="10"></circle>
              <line x1="12" y1="8" x2="12" y2="12"></line>
              <line x1="12" y1="16" x2="12.01" y2="16"></line>
            </svg>
            <p class="text-sm text-red-500">{error}</p>
          </div>
        {/if}

        <Button type="submit" class="w-full py-3 text-base" disabled={isVerifying}>
          {#if isVerifying}
            <svg
              class="animate-spin -ml-1 mr-2 h-5 w-5"
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
            >
              <circle
                class="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                stroke-width="4"
              ></circle>
              <path
                class="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              ></path>
            </svg>
            Verifying...
          {:else}
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              class="mr-2"
            >
              <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"></path>
              <polyline points="10 17 15 12 10 7"></polyline>
              <line x1="15" y1="12" x2="3" y2="12"></line>
            </svg>
            Connect
          {/if}
        </Button>
      </form>
    </div>

    <!-- Footer -->
    <div class="mt-8 text-center">
      <p class="text-xs text-muted-foreground">Running in self-hosted server mode</p>
    </div>
  </div>
</div>
