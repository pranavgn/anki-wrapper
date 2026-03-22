<script lang="ts">
  let { 
    reviewedCount = 0,
    onNavigateToBrowser
  }: { 
    reviewedCount?: number;
    onNavigateToBrowser: () => void;
  } = $props();

  let isAnimating = $state(false);

  export function triggerReceiveAnimation() {
    isAnimating = true;
    setTimeout(() => {
      isAnimating = false;
    }, 450);
  }
</script>

<button
  onclick={onNavigateToBrowser}
  class="fixed bottom-6 right-6 z-40 neu-raised neu-btn cursor-pointer"
  style="
    background: var(--bg-card);
    box-shadow: var(--neu-up);
    border-radius: var(--radius-md);
    width: 60px;
    height: 60px;
    border: none;
    padding: 0;
    {isAnimating ? 'animation: deckReceive 0.45s cubic-bezier(0.34,1.56,0.64,1);' : ''}
  "
  title="Browse cards"
>
  <!-- Stacked card layers -->
  <div class="relative w-full h-full flex items-center justify-center">
    <!-- Bottom layer -->
    <div 
      class="absolute w-10 h-12 rounded-lg"
      style="
        background: var(--bg-subtle);
        transform: translateY(4px) rotate(3deg);
        opacity: 0.4;
      "
    ></div>
    <!-- Middle layer -->
    <div 
      class="absolute w-10 h-12 rounded-lg"
      style="
        background: var(--bg-card-raised);
        transform: translateY(2px) rotate(-1.5deg);
        opacity: 0.6;
      "
    ></div>
    <!-- Top layer -->
    <div 
      class="absolute w-10 h-12 rounded-lg flex flex-col items-center justify-center gap-1"
      style="
        background: white;
        box-shadow: 0 1px 3px rgba(0,0,0,0.1);
      "
    >
      <div class="w-6 h-0.5 rounded" style="background: var(--text-muted); opacity: 0.5;"></div>
      <div class="w-5 h-0.5 rounded" style="background: var(--text-muted); opacity: 0.5;"></div>
      <div class="w-6 h-0.5 rounded" style="background: var(--text-muted); opacity: 0.5;"></div>
    </div>
  </div>

  <!-- Badge -->
  {#if reviewedCount > 0}
    <div 
      class="absolute -top-1 -right-1 w-5 h-5 rounded-full flex items-center justify-center"
      style="
        background: var(--accent);
        font-family: var(--sans);
        font-size: 10px;
        font-weight: 600;
        color: white;
      "
    >
      {reviewedCount}
    </div>
  {/if}
</button>

<style>
  @keyframes deckReceive {
    0% { transform: scale(1); }
    30% { transform: scale(1.14); }
    60% { transform: scale(0.95); }
    100% { transform: scale(1); }
  }
</style>
