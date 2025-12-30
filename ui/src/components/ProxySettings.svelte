<script>
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { getProxyUrl, setProxyUrl } from '$lib/api.js';

  // Check if running in Tauri (desktop app)
  const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

  let proxyUrl = $state(getProxyUrl());
  let showHelp = $state(false);

  function saveProxy() {
    setProxyUrl(proxyUrl);
    alert('Proxy URL saved! Reload the page for changes to take effect.');
  }

  function clearProxy() {
    proxyUrl = '';
    setProxyUrl('');
    alert('Proxy cleared! Reload the page for changes to take effect.');
  }
</script>

{#if !isTauri}
<Card class="p-3 mb-3">
  <div class="flex items-center justify-between mb-3">
    <h2 class="text-primary text-lg font-semibold">🌐 CORS Proxy (Optional)</h2>
    <button
      class="text-muted-foreground hover:text-foreground text-sm"
      onclick={() => (showHelp = !showHelp)}
    >
      {showHelp ? '▼ Hide Help' : '▶ Show Help'}
    </button>
  </div>

  {#if showHelp}
    <div class="bg-muted/50 p-3 rounded-lg mb-3 text-sm">
      <p class="mb-2">
        <strong>Why do I need this?</strong> Most BitTorrent trackers don't support CORS, which
        prevents the web browser from making requests to them.
      </p>
      <p class="mb-2">
        <strong>Solution:</strong> Deploy a free Cloudflare Worker as a CORS proxy. See our
        <a
          href="https://github.com/takitsu21/rustatio/blob/main/WEB_VERSION.md"
          target="_blank"
          class="text-primary hover:underline"
        >
          setup guide
        </a>
        for step-by-step instructions (takes 5 minutes).
      </p>
      <p class="mb-2">
        <strong>Example Worker URL:</strong>
        <code class="bg-background px-2 py-1 rounded text-xs">
          https://rustatio-cors-proxy.yourname.workers.dev
        </code>
      </p>
      <p class="text-yellow-600 dark:text-yellow-400">
        ⚠️ Without a proxy, only CORS-enabled trackers will work.
      </p>
    </div>
  {/if}

  <div class="flex flex-col gap-2">
    <label for="proxy-url" class="text-sm font-medium">Proxy URL (leave empty to disable)</label>
    <input
      id="proxy-url"
      type="url"
      bind:value={proxyUrl}
      placeholder="https://your-worker.workers.dev"
      class="w-full px-3 py-2 bg-background border border-border rounded-md text-sm focus:outline-none focus:ring-2 focus:ring-primary"
    />
    <div class="flex gap-2">
      <Button onclick={saveProxy} class="flex-1">
        {#snippet children()}
          💾 Save Proxy
        {/snippet}
      </Button>
      {#if proxyUrl}
        <Button onclick={clearProxy} class="flex-1 bg-destructive hover:bg-destructive/90">
          {#snippet children()}
            🗑️ Clear
          {/snippet}
        </Button>
      {/if}
    </div>
    {#if proxyUrl}
      <p class="text-xs text-green-600 dark:text-green-400">
        ✅ Proxy configured: All tracker requests will be routed through this proxy
      </p>
    {:else}
      <p class="text-xs text-yellow-600 dark:text-yellow-400">
        ⚠️ No proxy configured: Only CORS-enabled trackers will work
      </p>
    {/if}
  </div>
</Card>
{/if}
