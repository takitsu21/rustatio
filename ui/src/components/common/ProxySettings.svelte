<script>
  import Button from '$lib/components/ui/button.svelte';
  import { getProxyUrl, setProxyUrl, getRunMode } from '$lib/api.js';
  import { Globe, Save, Trash2, AlertTriangle, CheckCircle } from '@lucide/svelte';

  // Only show proxy settings in WASM mode (GitHub Pages)
  // Desktop (Tauri) and Server modes don't have CORS limitations
  let runMode = $derived(getRunMode());

  let proxyUrl = $state(getProxyUrl());
  let showHelp = $state(false);
  let feedback = $state('');

  function announce(message) {
    feedback = message;
    setTimeout(() => {
      if (feedback === message) feedback = '';
    }, 4000);
  }

  function saveProxy() {
    setProxyUrl(proxyUrl);
    announce('Proxy saved. Reload Rustatio to apply the change.');
  }

  function clearProxy() {
    proxyUrl = '';
    setProxyUrl('');
    announce('Proxy cleared. Reload Rustatio to apply the change.');
  }
</script>

<!-- Only show in WASM mode (GitHub Pages) - Desktop and Server don't need CORS proxy -->
{#if runMode === 'wasm'}
  <section class="workspace-section p-4">
    <div class="flex items-center justify-between mb-3">
      <h3 class="flex items-center gap-2 font-semibold text-foreground">
        <Globe size={17} class="text-primary" /> Web tracker proxy
      </h3>
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
          <strong>Why do I need this?</strong> Most BitTorrent trackers don't support CORS, which prevents
          the web browser from making requests to them.
        </p>
        <p class="mb-2">
          <strong>Solution 1 (Recommended):</strong> Use the
          <a
            href="https://github.com/takitsu21/rustatio/releases/latest"
            target="_blank"
            class="text-primary hover:underline font-semibold"
          >
            desktop app
          </a>
          which has no CORS limitations and works with all trackers out of the box.
        </p>
        <p class="mb-2">
          <strong>Solution 2:</strong> Deploy a free Cloudflare Worker as a CORS proxy. See our
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
        <p class="text-stat-danger flex items-center gap-1.5">
          <AlertTriangle size={16} class="flex-shrink-0" /> Without a proxy, only CORS-enabled trackers
          will work.
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
            <span class="flex items-center gap-1.5"><Save size={16} /> Save Proxy</span>
          {/snippet}
        </Button>
        {#if proxyUrl}
          <Button
            onclick={clearProxy}
            class="flex-1 bg-stat-danger hover:bg-stat-danger/90 text-white shadow-sm"
          >
            {#snippet children()}
              <span class="flex items-center gap-1.5"><Trash2 size={16} /> Clear</span>
            {/snippet}
          </Button>
        {/if}
      </div>
      {#if proxyUrl}
        <p class="text-xs text-stat-upload flex items-center gap-1.5">
          <CheckCircle size={14} class="flex-shrink-0" /> Proxy configured: All tracker requests will
          be routed through this proxy
        </p>
      {:else}
        <p class="text-xs text-stat-ratio flex items-center gap-1.5">
          <AlertTriangle size={14} class="flex-shrink-0" /> No proxy configured: Only CORS-enabled trackers
          will work
        </p>
      {/if}
      {#if feedback}
        <p class="text-xs text-primary" role="status" aria-live="polite">{feedback}</p>
      {/if}
    </div>
  </section>
{/if}
