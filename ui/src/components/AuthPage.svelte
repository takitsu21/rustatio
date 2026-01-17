<script>
  import { onMount } from 'svelte';
  import { getAuthToken, setAuthToken, verifyAuthToken, clearAuthToken } from '../lib/api.js';
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
  } from '../lib/themeStore.svelte.js';
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
        <span class="transition-transform {getShowThemeDropdown() ? 'rotate-180' : ''}">
          <ChevronDown size={14} />
        </span>
      </button>
      {#if getShowThemeDropdown()}
        <div
          class="absolute top-[calc(100%+0.5rem)] right-0 bg-card text-card-foreground border border-border/50 rounded-xl shadow-2xl p-1.5 min-w-[200px] max-h-[400px] overflow-y-auto z-50 backdrop-blur-xl animate-in fade-in slide-in-from-top-2 duration-200"
        >
          {#each Object.entries(THEME_CATEGORIES) as [categoryId, category] (categoryId)}
            <!-- Category Header -->
            <div
              class="px-3 py-1.5 text-xs font-semibold text-muted-foreground uppercase tracking-wider {categoryId !==
              'default'
                ? 'mt-2 border-t border-border pt-2'
                : ''}"
            >
              {category.name}
            </div>

            {#each category.themes as themeId (themeId)}
              {@const themeOption = THEMES[themeId]}
              <button
                class="w-full flex items-center gap-3 px-3 py-2 border-none cursor-pointer rounded-lg transition-all {getTheme() ===
                themeOption.id
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : 'bg-transparent text-card-foreground hover:bg-secondary/80'}"
                onclick={() => selectTheme(themeOption.id)}
              >
                <ThemeIcon theme={themeOption.id} />
                <div class="flex-1 text-left">
                  <span class="text-sm font-medium">{themeOption.name}</span>
                  {#if themeOption.description}
                    <span class="block text-xs opacity-70">{themeOption.description}</span>
                  {/if}
                </div>
                {#if getTheme() === themeOption.id}
                  <Check size={16} strokeWidth={2.5} />
                {/if}
              </button>
            {/each}
          {/each}
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
          <div class="w-10 h-10 bg-stat-ratio/10 rounded-xl flex items-center justify-center">
            <Lock size={20} class="text-stat-ratio" />
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
              <KeyRound size={18} class="text-muted-foreground" />
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
          <div
            class="p-4 rounded-xl bg-stat-leecher/10 border border-stat-leecher/20 flex items-start gap-3"
          >
            <AlertCircle size={20} class="text-stat-leecher flex-shrink-0 mt-0.5" />
            <p class="text-sm text-stat-leecher">{error}</p>
          </div>
        {/if}

        <Button type="submit" class="w-full py-3 text-base" disabled={isVerifying}>
          {#if isVerifying}
            <Loader2 size={20} class="animate-spin -ml-1 mr-2" />
            Verifying...
          {:else}
            <LogIn size={18} class="mr-2" />
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
