<script>
  import { cn } from '$lib/core/utils.js';

  let {
    class: className = '',
    type = 'text',
    value = $bindable(),
    disabled = false,
    placeholder = '',
    ...props
  } = $props();
</script>

<input
  {type}
  bind:value
  {disabled}
  {placeholder}
  class={cn(
    'flex h-8 w-full rounded-md border border-input bg-background px-2.5 py-1 text-xs text-foreground placeholder:text-muted-foreground focus-visible:border-ring focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring/50 disabled:cursor-not-allowed disabled:opacity-50',
    className
  )}
  {...props}
/>
