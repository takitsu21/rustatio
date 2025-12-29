<script>
  let { isRunning, nextUpdateIn } = $props();
</script>

<section class="text-center my-3">
  {#if isRunning}
    <div
      class="inline-block bg-muted border-2 border-border rounded-lg px-4 py-2 text-muted-foreground text-sm"
    >
      Next update in: <strong class="text-base text-primary font-bold ml-1">{nextUpdateIn}s</strong>
    </div>
  {/if}
</section>
