<script>
  let { tracker, iconUrl = '', initial = '', class: className = '' } = $props();

  let failed = $state(false);

  function handleError() {
    failed = true;
  }
</script>

{#if iconUrl && !failed}
  <img
    src={iconUrl}
    alt=""
    class={`h-4 w-4 rounded-sm object-cover bg-muted flex-shrink-0 ${className}`}
    loading="lazy"
    onerror={handleError}
  />
{:else}
  <span
    class={`inline-flex h-4 w-4 items-center justify-center rounded-sm bg-muted text-[9px] font-semibold uppercase text-muted-foreground shrink-0 ${className}`}
    aria-hidden="true"
  >
    {initial || tracker?.charAt(0)?.toUpperCase() || '?'}
  </span>
{/if}
