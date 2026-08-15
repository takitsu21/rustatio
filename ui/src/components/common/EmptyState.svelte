<script>
  let { icon, title, description, steps = [], action } = $props();
</script>

<div class="flex min-h-[24rem] items-center justify-center px-5 py-10">
  <div class="w-full max-w-xl text-center">
    {#if icon}
      <div
        class="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-lg border border-primary/25 bg-primary/10 text-primary"
      >
        {@render icon()}
      </div>
    {/if}
    <h2 class="text-xl font-bold tracking-tight text-foreground">{title}</h2>
    <p class="mx-auto mt-2 max-w-md text-sm leading-6 text-muted-foreground">{description}</p>
    {#if steps.length}
      <ol class="mx-auto mt-5 grid max-w-lg gap-2 text-left sm:grid-cols-3">
        {#each steps as step, index (step)}
          <li class="quiet-surface flex items-start gap-2 p-3 text-xs text-muted-foreground">
            <span
              class="mono-data flex h-5 w-5 shrink-0 items-center justify-center rounded-full bg-primary text-[10px] font-bold text-primary-foreground"
            >
              {index + 1}
            </span>
            <span class="pt-0.5">{step}</span>
          </li>
        {/each}
      </ol>
    {/if}
    {#if action}<div class="mt-5 flex justify-center">{@render action()}</div>{/if}
  </div>
</div>
