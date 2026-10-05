<script>
  import { cn } from '$lib/core/utils.js';
  import {
    Circle,
    Pause,
    Square,
    Moon,
    LoaderCircle,
    AlertTriangle,
    Rocket,
    Check,
  } from '@lucide/svelte';
  import { getStateMeta, getStatusTypeMeta, getTone } from '$lib/core/statusMeta.js';

  let { state = null, type = null, label = null, size = 'sm', class: className = '' } = $props();

  const ICONS = {
    circle: Circle,
    pause: Pause,
    square: Square,
    moon: Moon,
    loader: LoaderCircle,
    alert: AlertTriangle,
    rocket: Rocket,
    check: Check,
  };

  let meta = $derived(state != null ? getStateMeta(state) : getStatusTypeMeta(type));
  let tone = $derived(getTone(meta.tone));
  let Icon = $derived(meta.icon && meta.icon !== 'circle' ? ICONS[meta.icon] : null);
  let iconSize = $derived(size === 'xs' ? 9 : 10);
  let text = $derived(label ?? meta.label);
</script>

<span
  class={cn(
    'inline-flex items-center gap-1 border px-1.5 font-semibold uppercase tracking-[0.08em] whitespace-nowrap',
    size === 'xs' ? 'h-4 text-[0.5625rem]' : 'h-5 text-[0.625rem]',
    tone.border,
    tone.bg,
    tone.text,
    className
  )}
  title={text}
>
  {#if meta.spin}
    <LoaderCircle size={iconSize} class="animate-spin" />
  {:else if Icon}
    <Icon size={iconSize} class="flex-shrink-0" />
  {:else}
    <span class={cn('h-1.5 w-1.5 flex-shrink-0 rounded-full', tone.dot)}></span>
  {/if}
  <span>{text}</span>
</span>
