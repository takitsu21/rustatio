<script>
  import { cn } from '$lib/core/utils.js';

  let { class: className = '', children, ...props } = $props();
</script>

<div
  class={cn('rounded-md border border-border bg-card text-card-foreground', className)}
  {...props}
>
  {@render children?.()}
</div>
