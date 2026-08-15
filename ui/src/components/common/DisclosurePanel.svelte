<script>
  import { ChevronDown } from '@lucide/svelte';
  import { cn } from '$lib/utils.js';

  let {
    title,
    description = '',
    open = $bindable(false),
    badge = '',
    children,
    class: className = '',
  } = $props();
</script>

<section class={cn('workspace-section overflow-hidden', className)}>
  <button
    type="button"
    class="flex min-h-11 w-full items-center gap-3 px-3.5 py-2.5 text-left transition-colors hover:bg-muted/45"
    onclick={() => (open = !open)}
    aria-expanded={open}
  >
    <div class="min-w-0 flex-1">
      <div class="text-sm font-semibold text-foreground">{title}</div>
      {#if description}<div class="mt-0.5 text-xs text-muted-foreground">{description}</div>{/if}
    </div>
    {#if badge}
      <span
        class="rounded-full bg-muted px-2 py-0.5 text-[11px] font-semibold text-muted-foreground"
      >
        {badge}
      </span>
    {/if}
    <ChevronDown
      size={16}
      class={cn('shrink-0 text-muted-foreground transition-transform', open && 'rotate-180')}
    />
  </button>
  {#if open}
    <div class="border-t border-border/70 p-3.5">{@render children()}</div>
  {/if}
</section>
