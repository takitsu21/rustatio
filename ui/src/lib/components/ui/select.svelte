<script>
  import { cn } from '$lib/core/utils.js';

  let {
    class: className = '',
    value = $bindable(),
    disabled = false,
    children,
    ...props
  } = $props();
</script>

<select
  bind:value
  {disabled}
  class={cn(
    'flex h-8 w-full items-center justify-between rounded-md border border-input bg-background px-2.5 py-1 text-xs text-foreground focus:border-ring focus:outline-none focus:ring-1 focus:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50',
    className
  )}
  {...props}
>
  {@render children?.()}
</select>
