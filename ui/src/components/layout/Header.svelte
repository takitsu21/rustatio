<script>
  import { Menu, Settings, Github, ChevronDown, Check } from '@lucide/svelte';
  import { cn } from '$lib/core/utils.js';
  import ThemeIcon from '../common/ThemeIcon.svelte';
  import DownloadButton from '../common/DownloadButton.svelte';
  import {
    THEMES,
    THEME_CATEGORIES,
    getTheme,
    getShowThemeDropdown,
    toggleThemeDropdown,
    selectTheme,
    getThemeName,
  } from '$lib/themes/themeStore.svelte.js';

  let { onToggleSidebar = () => {}, onOpenSettings = () => {} } = $props();

  const repository = 'https://github.com/takitsu21/rustatio';
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
</script>

<header class="flex h-9 shrink-0 items-center gap-2 border-b border-border bg-card px-2">
  <!-- Mobile menu -->
  <button
    class="inline-flex h-6 w-6 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground lg:hidden cursor-pointer"
    onclick={onToggleSidebar}
    aria-label="Toggle menu"
  >
    <Menu size={15} />
  </button>

  <!-- Identity -->
  <div class="flex min-w-0 items-center gap-2">
    <img src="/favicon-32x32.png" alt="Rustatio" width="16" height="16" class="flex-shrink-0" />
    <span class="truncate text-xs font-semibold text-foreground">Rustatio</span>
    <span class="hidden truncate text-[0.625rem] text-muted-foreground md:inline"
      >BitTorrent ratio faker</span
    >
  </div>

  <!-- Global actions -->
  <div class="ml-auto flex items-center gap-0.5">
    {#if !isTauri}
      <div class="hidden sm:block">
        <DownloadButton />
      </div>
      <span class="mx-1 hidden h-3 w-px bg-border sm:block" aria-hidden="true"></span>
    {/if}

    <a
      href={repository}
      target="_blank"
      rel="noopener noreferrer"
      class="inline-flex h-6 w-6 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      title="View Rustatio on GitHub"
      aria-label="GitHub repository"
    >
      <Github size={14} />
    </a>

    <button
      class="inline-flex h-6 w-6 items-center justify-center text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
      onclick={onOpenSettings}
      title="Settings"
      aria-label="Open settings"
    >
      <Settings size={14} />
    </button>

    <!-- Theme selector -->
    <div class="relative theme-selector">
      <button
        onclick={toggleThemeDropdown}
        class="inline-flex h-6 items-center gap-1 px-1 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground cursor-pointer"
        title="Theme: {getThemeName(getTheme())}"
        aria-label="Toggle theme menu"
      >
        <ThemeIcon theme={getTheme()} />
        <ChevronDown
          size={11}
          class={cn('transition-transform', getShowThemeDropdown() && 'rotate-180')}
        />
      </button>

      {#if getShowThemeDropdown()}
        <div
          class="absolute right-0 top-[calc(100%+0.25rem)] z-50 max-h-[26.25rem] min-w-[13.125rem] overflow-y-auto border border-border bg-popover p-1 text-popover-foreground"
        >
          {#each Object.entries(THEME_CATEGORIES) as [categoryId, category] (categoryId)}
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
</header>
