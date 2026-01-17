<script>
  import { Menu } from '@lucide/svelte';

  let { onToggleSidebar } = $props();
</script>

<header class="py-3 px-4">
  <div class="flex justify-between items-center">
    <div class="flex items-center gap-3">
      <!-- Mobile Menu Toggle Button -->
      <button
        class="lg:hidden p-2 bg-primary text-primary-foreground rounded-md shadow-lg"
        onclick={onToggleSidebar}
        aria-label="Toggle menu"
      >
        <Menu size={20} />
      </button>

      <div class="flex flex-col">
        <h1 class="text-2xl font-bold text-foreground tracking-tight">Rustatio</h1>
        <p class="text-xs text-muted-foreground mt-0.5">Modern BitTorrent Ratio Faker</p>
      </div>
    </div>
  </div>
</header>
