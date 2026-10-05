<script>
  import { cn } from '$lib/core/utils.js';

  let {
    variant = 'default',
    size = 'default',
    class: className = '',
    onclick = undefined,
    disabled = false,
    type = 'button',
    children,
    ...props
  } = $props();

  const variants = {
    default: 'bg-primary text-primary-foreground hover:bg-primary/85',
    destructive: 'bg-destructive text-destructive-foreground hover:bg-destructive/85',
    outline: 'border-border bg-transparent text-foreground hover:bg-muted',
    secondary: 'bg-secondary text-secondary-foreground hover:bg-secondary/70',
    ghost: 'text-muted-foreground hover:bg-muted hover:text-foreground',
    link: 'text-primary underline-offset-4 hover:underline',
  };

  const sizes = {
    default: 'h-8 px-3 text-xs',
    sm: 'h-7 px-2 text-[0.6875rem]',
    lg: 'h-10 px-4 text-sm',
    icon: 'h-8 w-8',
  };
</script>

<button
  {type}
  class={cn(
    'inline-flex select-none items-center justify-center gap-1.5 whitespace-nowrap rounded-md border border-transparent font-medium transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:pointer-events-none disabled:opacity-40 cursor-pointer',
    variants[variant],
    sizes[size],
    className
  )}
  {disabled}
  {onclick}
  {...props}
>
  {@render children?.()}
</button>
