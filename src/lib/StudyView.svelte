<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";

  // Props using Svelte 5 runes
  interface Props {
    deckId: number;
    deckName: string;
    onExit: () => void;
  }

  let { deckId, deckName, onExit }: Props = $props();

  // State
  let isLoading = $state(true);
  let error = $state("");
  let currentCard = $state<any>(null);
  let cardState = $state<'idle' | 'flipped' | 'leaving-left' | 'leaving-right' | 'leaving-up' | 'leaving-down' | 'entering'>('idle');
  let flagBorderColor = $state('#3b82f6'); // Default blue color

  // Load first card on mount
  onMount(async () => {
    const tauriCheck = await isTauri();
    if (!tauriCheck) {
      error = "This feature requires Tauri desktop environment";
      isLoading = false;
      return;
    }
    
    console.log("Loading card for deckId:", deckId);
    try {
      const card = await invoke<any>("get_next_card", { deckId });
      console.log("Card received:", card);
      currentCard = card;
    } catch (err) {
      console.error("Error loading card:", err);
      error = String(err);
    } finally {
      isLoading = false;
    }
  });

  onDestroy(() => {
  });

  // Derived style for active card
  let activeCardStyle = $derived.by(() => {
    // Build transform
    let transform = '';
    if (cardState === 'flipped') transform = 'rotateY(180deg)';
    else if (cardState === 'leaving-left') transform = 'translateX(-120%) rotate(-8deg)';
    else if (cardState === 'leaving-right') transform = 'translateX(120%) rotate(8deg)';
    else if (cardState === 'leaving-up') transform = 'translateY(-60px)';
    else if (cardState === 'leaving-down') transform = 'translateY(60px)';
    else if (cardState === 'entering') transform = 'scale(0.93)';

    // Build opacity
    let opacity = '';
    if (cardState === 'leaving-up' || cardState === 'leaving-down' || cardState === 'entering') {
      opacity = 'opacity: 0;';
    }

    // Build transition — filter out empty strings so we never get "transition: , , ;"
    const transitionParts: string[] = [];
    if (cardState === 'flipped') {
      transitionParts.push('transform 480ms cubic-bezier(0.4, 0, 0.2, 1)');
    }
    if (cardState.startsWith('leaving')) {
      transitionParts.push('transform 280ms ease-in', 'opacity 280ms ease-in');
    }
    if (cardState === 'entering') {
      transitionParts.push('transform 240ms ease-out', 'opacity 240ms ease-out');
    }
    // For idle state, use a default transition so entering→idle animates smoothly
    if (cardState === 'idle') {
      transitionParts.push('transform 240ms ease-out', 'opacity 240ms ease-out');
    }
    const transition = transitionParts.length > 0
      ? `transition: ${transitionParts.join(', ')};`
      : '';

    return `transform-style: preserve-3d; z-index: 2; ${transform ? `transform: ${transform};` : ''} ${opacity} ${transition} border-top: 3px solid ${flagBorderColor}`;
  });
</script>

<div class="absolute inset-0 bg-bg-base flex flex-col overflow-hidden z-50">
  <!-- Header -->
  <header class="relative z-10 flex justify-between items-center px-6 py-4">
    <div class="flex items-center gap-2">
      <button
        onclick={onExit}
        class="flex items-center gap-2 text-text-secondary hover:text-text-primary transition-colors"
        aria-label="Back to dashboard"
      >
        <svg class="h-6 w-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
        <span class="text-sm font-medium">Back</span>
      </button>
    </div>
  </header>

  <!-- Main Content -->
  <main class="flex-1 flex items-center justify-center p-6">
    <div class="w-full max-w-[680px]">
      <!-- Loading State -->
      {#if isLoading}
        <div class="bg-bg-card rounded-3xl shadow-warm p-10 text-center border border-border">
          <div class="animate-spin rounded-full h-12 w-12 border-4 border-accent border-t-transparent mx-auto mb-4"></div>
          <p class="text-text-secondary">Loading card...</p>
        </div>
      
      <!-- Error State -->
      {:else if error}
        <div class="bg-bg-card rounded-3xl shadow-warm p-10 text-center border border-border">
          <div class="w-16 h-16 bg-danger/10 rounded-full flex items-center justify-center mx-auto mb-4">
            <svg class="h-8 w-8 text-danger" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <h2 class="text-xl font-semibold text-text-primary mb-2">Error</h2>
          <p class="text-text-secondary mb-6">{error}</p>
          <button
            onclick={onExit}
            class="px-6 py-3 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors font-medium cursor-pointer active:scale-95"
          >
            Back to Dashboard
          </button>
        </div>
      
      <!-- No Cards Left -->
      {:else if !currentCard}
        <div class="bg-bg-card rounded-3xl shadow-warm p-10 text-center border border-border animate-in zoom-in-95 duration-300">
          <svg class="w-20 h-20 text-text-secondary opacity-40 mx-auto mb-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <circle cx="12" cy="12" r="10" stroke-width="1.5" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12l2 2 4-4" />
          </svg>
          <h2 class="text-2xl font-semibold text-text-primary mb-2">All done for today!</h2>
          <p class="text-text-secondary mb-6">
            You've reviewed all cards. Great job!
          </p>
          <button
            onclick={onExit}
            class="px-6 py-3 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors font-medium cursor-pointer active:scale-95"
          >
            Back to Decks
          </button>
        </div>
      
      <!-- Card Display -->
      {:else}
        <div class="bg-bg-card rounded-3xl shadow-warm border border-border overflow-visible relative" style={activeCardStyle}>
          <div class="p-10">
            <h2 class="text-xl font-semibold text-text-primary mb-4">Card Content</h2>
            
            <div class="mb-4">
              <h3 class="font-medium text-text-secondary mb-2">Front Side:</h3>
              <div class="bg-bg-base p-4 rounded-xl">
                {@html currentCard.front}
              </div>
            </div>
            
            <div class="mb-6">
              <h3 class="font-medium text-text-secondary mb-2">Back Side:</h3>
              <div class="bg-bg-base p-4 rounded-xl">
                {@html currentCard.back}
              </div>
            </div>
            
            <button
              onclick={() => onExit()}
              class="px-6 py-3 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors font-medium cursor-pointer active:scale-95"
            >
              Exit Study
            </button>
          </div>
        </div>
      {/if}
    </div>
  </main>
</div>
