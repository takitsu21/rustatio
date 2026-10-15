<script>
  import { tick } from 'svelte';
  import { Check, ChevronDown } from '@lucide/svelte';
  import { cn } from '$lib/core/utils.js';
  import ThemeIcon from './ThemeIcon.svelte';
  import { THEMES, groupThemes } from '$lib/themes/themeStore.svelte.js';

  let {
    id = undefined,
    value,
    themes,
    onchange = () => {},
    label = 'Theme',
    class: className = '',
  } = $props();

  let open = $state(false);
  let menuStyle = $state('');
  let containerEl = $state(null);
  let buttonEl = $state(null);
  let menuEl = $state(null);

  let groups = $derived(groupThemes(themes));

  function menuOptions() {
    return [...(menuEl?.querySelectorAll('[role="option"]') ?? [])];
  }

  function close() {
    open = false;
  }

  function positionMenu() {
    if (!buttonEl) return;

    // Fixed positioning keeps the menu visible inside the scrolling settings
    // dialog; it is re-anchored to the button on ancestor scrolls.
    const rect = buttonEl.getBoundingClientRect();
    const estimated = Math.min(300, groups.length * 26 + themes.length * 28 + 8);
    const below = window.innerHeight - rect.bottom - 8;
    const up = below < estimated && rect.top > below;

    menuStyle = up
      ? `left: ${rect.left}px; width: ${rect.width}px; bottom: ${window.innerHeight - rect.top + 4}px;`
      : `left: ${rect.left}px; width: ${rect.width}px; top: ${rect.bottom + 4}px;`;
  }

  function isButtonVisible() {
    if (!buttonEl) return false;
    const rect = buttonEl.getBoundingClientRect();
    return rect.bottom > 0 && rect.top < window.innerHeight;
  }

  function openMenu() {
    if (!buttonEl) return;
    positionMenu();
    open = true;
  }

  function toggle() {
    if (open) {
      close();
    } else {
      openMenu();
    }
  }

  function choose(themeId) {
    onchange(themeId);
    close();
    buttonEl?.focus();
  }

  async function onKeydown(event) {
    if (event.key === 'Escape') {
      if (!open) return;
      close();
      buttonEl?.focus();
      event.stopPropagation();
      return;
    }

    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();

    const step = event.key === 'ArrowDown' ? 1 : -1;

    if (!open) {
      openMenu();
      await tick();
      const options = menuOptions();
      const selected = options.find(option => option.getAttribute('aria-selected') === 'true');
      (selected ?? options[0])?.focus();
      return;
    }

    const options = menuOptions();
    if (!options.length) return;

    const index = options.indexOf(document.activeElement);
    const next =
      index < 0
        ? options[step > 0 ? 0 : options.length - 1]
        : options[(index + step + options.length) % options.length];

    next?.focus();
  }

  function onFocusOut(event) {
    if (!open) return;

    // Ignore focus moving to nothing (clicking the menu padding or a section
    // header); only close when focus lands on another control outside.
    const next = event.relatedTarget;
    if (!next || containerEl?.contains(next)) return;

    close();
  }

  $effect(() => {
    if (!open) return;

    const onPointerDown = event => {
      if (containerEl && !containerEl.contains(event.target)) close();
    };
    const onScroll = event => {
      // The menu itself must stay scrollable; follow the button for ancestor
      // scrolls and close once the button leaves the viewport.
      if (menuEl?.contains(event.target)) return;
      positionMenu();
      if (!isButtonVisible()) close();
    };
    const onResize = () => close();

    window.addEventListener('pointerdown', onPointerDown, true);
    window.addEventListener('scroll', onScroll, true);
    window.addEventListener('resize', onResize);

    return () => {
      window.removeEventListener('pointerdown', onPointerDown, true);
      window.removeEventListener('scroll', onScroll, true);
      window.removeEventListener('resize', onResize);
    };
  });
</script>

<div class={cn('relative', className)} bind:this={containerEl} onfocusout={onFocusOut}>
  <button
    {id}
    type="button"
    bind:this={buttonEl}
    onclick={toggle}
    onkeydown={onKeydown}
    aria-haspopup="listbox"
    aria-expanded={open}
    class="flex h-8 w-full cursor-pointer items-center gap-2 border border-input bg-background px-2.5 text-xs text-foreground focus:border-ring focus:outline-none"
  >
    <ThemeIcon theme={value} />
    <span class="flex-1 truncate text-left">{THEMES[value]?.name ?? ''}</span>
    <ChevronDown size={14} class="text-muted-foreground" />
  </button>

  {#if open}
    <div
      bind:this={menuEl}
      style={menuStyle}
      role="listbox"
      aria-label={label}
      tabindex="-1"
      onkeydown={onKeydown}
      class="fixed z-[90] max-h-72 overflow-y-auto border border-border bg-popover p-1 text-popover-foreground"
    >
      {#each groups as group, index (group.id)}
        <div
          class="px-2 py-1 text-[0.625rem] font-semibold uppercase tracking-[0.14em] text-muted-foreground {index >
          0
            ? 'mt-1 border-t border-border pt-1.5'
            : ''}"
        >
          {group.name}
        </div>
        {#each group.ids as themeId (themeId)}
          <button
            type="button"
            role="option"
            aria-selected={themeId === value}
            class="flex w-full cursor-pointer items-center gap-2 px-2 py-1.5 text-left text-xs transition-colors {themeId ===
            value
              ? 'bg-selected text-foreground'
              : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
            onclick={() => choose(themeId)}
          >
            <ThemeIcon theme={themeId} />
            <span class="flex-1">{THEMES[themeId].name}</span>
            {#if themeId === value}
              <Check size={12} strokeWidth={2.5} />
            {/if}
          </button>
        {/each}
      {/each}
    </div>
  {/if}
</div>
