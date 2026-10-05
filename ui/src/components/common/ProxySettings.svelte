<script>
  import Card from '$lib/components/ui/card.svelte';
  import Button from '$lib/components/ui/button.svelte';
  import { getProxyUrl, setProxyUrl, getRunMode } from '$lib/api.js';
  import { Globe, Save, Trash2, AlertTriangle, CheckCircle } from '@lucide/svelte';

  // Only show proxy settings in WASM mode (GitHub Pages)
  // Desktop (Tauri) and Server modes don't have CORS limitations
  let runMode = $derived(getRunMode());

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

<!-- Only show in WASM mode (GitHub Pages) - Desktop and Server don't need CORS proxy -->
{#if runMode === 'wasm'}
  <Card class="mb-2">
    <div class="flex h-8 items-center gap-2 border-b border-border px-2.5">
      <Globe size={12} class="text-muted-foreground" />
      <span
        class="flex-1 text-[0.625rem] font-semibold uppercase tracking-[0.16em] text-muted-foreground"
      >
        CORS proxy (optional)
      </span>
      <button
        class="cursor-pointer text-[0.625rem] text-muted-foreground transition-colors hover:text-foreground"
        onclick={() => (showHelp = !showHelp)}
      >
        {showHelp ? 'Hide help' : 'Show help'}
      </button>
    </div>

    <div class="p-2.5">
      {#if showHelp}
        <div class="mb-2.5 border border-border bg-muted/30 p-2.5 text-[0.6875rem] leading-5">
          <p class="mb-1.5">
            <strong>Why do I need this?</strong> Most BitTorrent trackers don't support CORS, which prevents
            the web browser from making requests to them.
          </p>
          <p class="mb-1.5">
            <strong>Solution 1 (recommended):</strong> Use the
            <a
              href="https://github.com/takitsu21/rustatio/releases/latest"
              target="_blank"
              class="font-semibold text-primary hover:underline"
            >
              desktop app
            </a>
            which has no CORS limitations and works with all trackers out of the box.
          </p>
          <p class="mb-1.5">
            <strong>Solution 2:</strong> Deploy a free
            <a
              href="https://developers.cloudflare.com/workers/"
              target="_blank"
              class="text-primary hover:underline"
            >
              Cloudflare Worker
            </a>
            as a CORS proxy.
          </p>
          <p class="mb-1.5">
            <strong>Example worker URL:</strong>
            <code class="border border-border bg-background px-1.5 py-0.5 text-[0.625rem]">
              https://rustatio-cors-proxy.yourname.workers.dev
            </code>
          </p>
          <p class="flex items-center gap-1.5 text-stat-ratio">
            <AlertTriangle size={12} class="flex-shrink-0" /> Without a proxy, only CORS-enabled trackers
            will work.
          </p>
        </div>
      {/if}

      <div class="flex flex-col gap-1.5">
        <label
          for="proxy-url"
          class="text-[0.625rem] uppercase tracking-wider text-muted-foreground"
          >Proxy URL (leave empty to disable)</label
        >
        <input
          id="proxy-url"
          type="url"
          bind:value={proxyUrl}
          placeholder="https://your-worker.workers.dev"
          class="h-8 w-full border border-input bg-background px-2.5 text-xs text-foreground placeholder:text-muted-foreground focus:border-ring focus:outline-none"
        />
        <div class="flex gap-1.5">
          <Button onclick={saveProxy} size="sm" class="flex-1">
            {#snippet children()}
              <Save size={12} /> Save proxy
            {/snippet}
          </Button>
          {#if proxyUrl}
            <Button
              onclick={clearProxy}
              size="sm"
              variant="outline"
              class="flex-1 text-stat-danger"
            >
              {#snippet children()}
                <Trash2 size={12} /> Clear
              {/snippet}
            </Button>
          {/if}
        </div>
        {#if proxyUrl}
          <p class="flex items-center gap-1.5 text-[0.625rem] text-stat-upload">
            <CheckCircle size={11} class="flex-shrink-0" /> Proxy configured: all tracker requests will
            be routed through this proxy
          </p>
        {:else}
          <p class="flex items-center gap-1.5 text-[0.625rem] text-stat-ratio">
            <AlertTriangle size={11} class="flex-shrink-0" /> No proxy configured: only CORS-enabled trackers
            will work
          </p>
        {/if}
      </div>
    </div>
  </Card>
{/if}
