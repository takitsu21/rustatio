<script>
  import { CircleHelp } from '@lucide/svelte';

  let { text, class: className = '' } = $props();
  let open = $state(false);
</script>

<span class={`relative inline-flex ${className}`}>
  <button
    type="button"
    class="inline-flex items-center text-muted-foreground/70 transition-colors hover:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 rounded-sm"
    aria-label={text}
    aria-expanded={open}
    onmouseenter={() => (open = true)}
    onmouseleave={() => (open = false)}
    onfocus={() => (open = true)}
    onblur={() => (open = false)}
  >
    <CircleHelp size={12} />
  </button>

  {#if open}
    <span
      role="tooltip"
      class="pointer-events-none absolute left-1/2 top-[calc(100%+0.4rem)] z-50 w-max max-w-[220px] -translate-x-1/2 rounded-lg border border-border/80 bg-popover px-2 py-1.5 text-[11px] leading-4 text-popover-foreground shadow-xl shadow-black/20"
    >
      {text}
    </span>
  {/if}
</span>
