<script>
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
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
        >
          <line x1="3" y1="12" x2="21" y2="12"></line>
          <line x1="3" y1="6" x2="21" y2="6"></line>
          <line x1="3" y1="18" x2="21" y2="18"></line>
        </svg>
      </button>
      
      <div class="flex flex-col">
        <h1 class="text-2xl font-bold text-foreground tracking-tight">Rustatio</h1>
        <p class="text-xs text-muted-foreground mt-0.5">Modern BitTorrent Ratio Faker</p>
      </div>
    </div>
  </div>
</header>
