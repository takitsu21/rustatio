<script>
  import { cn } from '$lib/utils.js';
  import Input from '$lib/components/ui/input.svelte';
  import Label from '$lib/components/ui/label.svelte';
  import ClientSelect from '../config/ClientSelect.svelte';
  import VersionSelect from '../config/VersionSelect.svelte';

  let {
    class: className = '',
    mode = 'client',
    clients = [],
    versions = [],
    selectedClient = $bindable(''),
    selectedVersion = $bindable(''),
    port = $bindable(6881),
    uploadRate = $bindable(0),
    downloadRate = $bindable(0),
    updateIntervalSeconds = $bindable(5),
    scrapeInterval = $bindable(60),
    showVersion = true,
    portDisabled = false,
    portInputClass = '',
    uploadLabel = 'Upload',
    downloadLabel = 'Download',
    refreshLabel = 'Refresh Interval',
    scrapeLabel = 'Scrape Interval',
    labelClass = 'mb-1.5 block text-xs text-muted-foreground',
    inputClass = '',
    inputStep = '0.1',
    showUnit = false,
    unit = 'KB/s',
    onClientChange = () => {},
    onVersionChange = () => {},
    onPortInput = () => {},
    onUploadInput = () => {},
    onDownloadInput = () => {},
    onRefreshInput = () => {},
    onScrapeInput = () => {},
    onRefreshBlur = () => {},
    onScrapeBlur = () => {},
    portHeaderExtra,
    portFooter,
  } = $props();
</script>

{#if mode === 'client'}
  <div class={cn('grid gap-3 md:grid-cols-3', className)}>
    <div>
      <Label class="mb-1.5 block text-xs text-muted-foreground">Type</Label>
      <ClientSelect {clients} bind:value={selectedClient} onchange={onClientChange} />
    </div>

    {#if showVersion}
      <div>
        <Label class="mb-1.5 block text-xs text-muted-foreground">Version</Label>
        <VersionSelect {versions} bind:value={selectedVersion} onchange={onVersionChange} />
      </div>
    {/if}

    <div>
      <div class="mb-1.5 flex items-center justify-between gap-2">
        <Label class="text-xs text-muted-foreground">Port</Label>
        {@render portHeaderExtra?.()}
      </div>
      <Input
        type="number"
        bind:value={port}
        disabled={portDisabled}
        min="1024"
        max="65535"
        class={portInputClass}
        oninput={onPortInput}
      />
      {@render portFooter?.()}
    </div>
  </div>
{:else if mode === 'rates'}
  <div class={cn('grid gap-3 md:grid-cols-2', className)}>
    <div>
      <Label class={labelClass}>{uploadLabel}</Label>
      {#if showUnit}
        <div class="flex items-center gap-2">
          <Input
            type="number"
            bind:value={uploadRate}
            min="0"
            step={inputStep}
            class={inputClass}
            oninput={onUploadInput}
          />
          <span class="text-sm text-muted-foreground">{unit}</span>
        </div>
      {:else}
        <Input
          type="number"
          bind:value={uploadRate}
          min="0"
          step={inputStep}
          class={inputClass}
          oninput={onUploadInput}
        />
      {/if}
    </div>

    <div>
      <Label class={labelClass}>{downloadLabel}</Label>
      {#if showUnit}
        <div class="flex items-center gap-2">
          <Input
            type="number"
            bind:value={downloadRate}
            min="0"
            step={inputStep}
            class={inputClass}
            oninput={onDownloadInput}
          />
          <span class="text-sm text-muted-foreground">{unit}</span>
        </div>
      {:else}
        <Input
          type="number"
          bind:value={downloadRate}
          min="0"
          step={inputStep}
          class={inputClass}
          oninput={onDownloadInput}
        />
      {/if}
    </div>
  </div>
{:else if mode === 'timing'}
  <div class={cn('grid gap-3 md:grid-cols-2', className)}>
    <div>
      <Label class={labelClass}>{refreshLabel}</Label>
      {#if showUnit}
        <div class="flex items-center gap-2">
          <Input
            type="number"
            bind:value={updateIntervalSeconds}
            min="1"
            max="300"
            step="1"
            class={inputClass}
            oninput={onRefreshInput}
            onblur={onRefreshBlur}
          />
          <span class="text-sm text-muted-foreground">{unit}</span>
        </div>
      {:else}
        <Input
          type="number"
          bind:value={updateIntervalSeconds}
          min="1"
          max="300"
          step="1"
          class={inputClass}
          oninput={onRefreshInput}
          onblur={onRefreshBlur}
        />
      {/if}
    </div>

    <div>
      <Label class={labelClass}>{scrapeLabel}</Label>
      {#if showUnit}
        <div class="flex items-center gap-2">
          <Input
            type="number"
            bind:value={scrapeInterval}
            min="10"
            max="3600"
            step="1"
            class={inputClass}
            oninput={onScrapeInput}
            onblur={onScrapeBlur}
          />
          <span class="text-sm text-muted-foreground">{unit}</span>
        </div>
      {:else}
        <Input
          type="number"
          bind:value={scrapeInterval}
          min="10"
          max="3600"
          step="1"
          class={inputClass}
          oninput={onScrapeInput}
          onblur={onScrapeBlur}
        />
      {/if}
    </div>
  </div>
{/if}
