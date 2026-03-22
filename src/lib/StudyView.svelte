<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import MiniDeck from "./MiniDeck.svelte";
  import { prefs } from "./prefs.svelte.ts";

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
  let currentCard = $state<{ card_id: number; note_id: number; front: string; back: string; flag: number; again_interval: string; hard_interval: string; good_interval: string; easy_interval: string } | null>(null);
  let isFlipped = $state(false);
  let cardAnimation = $state<'idle' | 'fly-out' | 'return'>('idle');
  let reviewedCount = $state(0);
  let totalCards = $state(0);
  let remainingCards = $state(0);
  let miniDeckRef: MiniDeck;

  // Load deck stats
  async function loadDeckStats() {
    try {
      const stats = await invoke<any>("get_deck_stats_for_review", { deckId });
      // Rust returns: new_cards, learn_cards, review_cards
      totalCards = (stats.new_cards ?? 0) + (stats.learn_cards ?? 0) + (stats.review_cards ?? 0);
      remainingCards = totalCards;
    } catch (e) {
      console.error("Error loading deck stats:", e);
    }
  }

  // Load next card
  async function loadNextCard() {
    isLoading = true;
    error = "";
    try {
      const card = await invoke<any>("get_next_card", { deckId });
      currentCard = card;
      isFlipped = false;
      cardAnimation = 'idle';
    } catch (e) {
      const msg = String(e);
      if (msg.includes("No cards left")) {
        currentCard = null;
        remainingCards = 0;
      } else {
        error = msg;
      }
    } finally {
      isLoading = false;
    }
  }

  // Load first card on mount
  onMount(async () => {
    const tauriCheck = await isTauri();
    if (!tauriCheck) {
      error = "This feature requires Tauri desktop environment";
      isLoading = false;
      return;
    }
    await loadDeckStats();
    await loadNextCard();
    
    // Add keyboard listener
    window.addEventListener('keydown', handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
  });

  function toggleFlip() {
    if (cardAnimation !== 'idle') return;
    isFlipped = !isFlipped;
  }

  async function answerCard(ease: number) {
    if (!currentCard || cardAnimation !== 'idle') return;

    const isAgain = ease === 1;
    cardAnimation = isAgain ? 'return' : 'fly-out';
    
    await new Promise(resolve => setTimeout(resolve, isAgain ? 650 : 700));

    try {
      await invoke("answer_card", {
        cardId: currentCard.card_id,  // CORRECT: card_id not id
        ease: ease
      });
      
      reviewedCount++;
      remainingCards = Math.max(0, remainingCards - 1);
      
      if (!isAgain && miniDeckRef) {
        miniDeckRef.triggerReceiveAnimation();
      }
      
      await loadNextCard();
    } catch (err) {
      console.error("Error answering card:", err);
      error = String(err);
      cardAnimation = 'idle';
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (isLoading) return;
    
    if (event.code === "Space" && !isFlipped && currentCard) {
      event.preventDefault();
      toggleFlip();
      return;
    }
    
    if (isFlipped && cardAnimation === 'idle') {
      if (["1","2","3","4"].includes(event.key)) {
        event.preventDefault();
        answerCard(parseInt(event.key));
      }
    }
    
    if (event.code === "Escape") {
      onExit();
    }
  }

  function navigateToBrowser() {
    window.dispatchEvent(new CustomEvent('navigate-to-browser'));
  }

  // Progress percentage
  const progressPercent = $derived(totalCards > 0 ? Math.round((reviewedCount / totalCards) * 100) : 0);

  // Get interval text for buttons
  function getIntervalText(ease: number): string {
    if (!currentCard) return '';
    if (ease === 1) return currentCard.again_interval || '';
    if (ease === 2) return currentCard.hard_interval || '';
    if (ease === 3) return currentCard.good_interval || '';
    if (ease === 4) return currentCard.easy_interval || '';
    return '';
  }
</script>

<div class="absolute inset-0 bg-bg-base flex flex-col overflow-hidden z-50">
  <!-- Progress Bar -->
  <div class="px-6 pt-4">
    <div 
      class="neu-pressed"
      style="
        background: var(--bg-deep);
        box-shadow: var(--neu-down);
        border-radius: 10px;
        height: 5px;
        overflow: hidden;
      "
    >
      <div 
        style="
          height: 100%;
          width: {progressPercent}%;
          background: linear-gradient(90deg, var(--accent), var(--success));
          border-radius: 10px;
          transition: width 0.5s ease-out;
        "
      ></div>
    </div>
  </div>

  <!-- Card Count -->
  <div class="flex justify-between items-center px-6 py-3">
    <span style="font-family: var(--sans); font-size: 13px; color: var(--text-muted);">
      {remainingCards} remaining
    </span>
    <span style="font-family: var(--sans); font-size: 13px; color: var(--text-muted);">
      {reviewedCount} reviewed
    </span>
  </div>

  <!-- Main Content -->
  <main class="flex-1 flex items-center justify-center p-6">
    <div class="w-full max-w-[680px]">
      <!-- Loading State -->
      {#if isLoading}
        <div 
          class="neu-raised text-center"
          style="
            background: var(--bg-card);
            box-shadow: var(--neu-up);
            border-radius: var(--radius-lg);
            padding: 40px;
          "
        >
          <div 
            class="animate-spin rounded-full h-12 w-12 border-4 mx-auto mb-4"
            style="border-color: var(--accent); border-top-color: transparent;"
          ></div>
          <p style="font-family: var(--sans); color: var(--text-secondary);">Loading card...</p>
        </div>
      
      <!-- Error State -->
      {:else if error}
        <div 
          class="neu-raised text-center"
          style="
            background: var(--bg-card);
            box-shadow: var(--neu-up);
            border-radius: var(--radius-lg);
            padding: 40px;
          "
        >
          <div 
            class="w-16 h-16 rounded-full flex items-center justify-center mx-auto mb-4"
            style="background: color-mix(in srgb, var(--danger) 10%, transparent);"
          >
            <svg class="h-8 w-8" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--danger);">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
          </div>
          <h2 style="font-family: var(--serif); font-size: 20px; font-weight: 600; color: var(--text-primary); margin-bottom: 8px;">Error</h2>
          <p style="font-family: var(--sans); color: var(--text-secondary); margin-bottom: 24px;">{error}</p>
          <button
            onclick={onExit}
            class="neu-raised neu-btn cursor-pointer"
            style="
              background: var(--accent);
              color: white;
              font-family: var(--sans);
              font-weight: 500;
              padding: 12px 24px;
              border-radius: var(--radius-md);
              border: none;
            "
          >
            Back to Deck
          </button>
        </div>
      
      <!-- No Cards Left -->
      {:else if !currentCard}
        <div 
          class="text-center"
          style="animation: scaleIn 0.3s ease-out;"
        >
          <div class="text-6xl mb-6">🎉</div>
          <h2 style="font-family: var(--serif); font-size: 30px; font-weight: 600; color: var(--text-primary); margin-bottom: 8px;">
            All done for today!
          </h2>
          <p style="font-family: var(--sans); font-size: 16px; color: var(--text-secondary); margin-bottom: 32px;">
            You've reviewed {reviewedCount} cards. Great job!
          </p>
          <button
            onclick={onExit}
            class="neu-raised neu-btn cursor-pointer"
            style="
              background: var(--accent);
              color: white;
              font-family: var(--sans);
              font-weight: 500;
              padding: 14px 32px;
              border-radius: var(--radius-md);
              border: none;
            "
          >
            Back to Deck
          </button>
        </div>
      
      <!-- Card Display -->
      {:else}
        <div class="relative">
          <!-- Ghost cards behind -->
          <div 
            class="absolute inset-0 neu-raised"
            style="
              background: var(--bg-card);
              box-shadow: var(--neu-up);
              border-radius: var(--radius-lg);
              opacity: 0.28;
              transform: translateY(11px) scale(0.93) rotate(2.2deg);
              z-index: 0;
            "
          ></div>
          <div 
            class="absolute inset-0 neu-raised"
            style="
              background: var(--bg-card);
              box-shadow: var(--neu-up);
              border-radius: var(--radius-lg);
              opacity: 0.5;
              transform: translateY(5.5px) scale(0.965) rotate(-1.1deg);
              z-index: 1;
            "
          ></div>

          <!-- Main card with flip -->
          <div 
            class="card-flip-container relative"
            style="z-index: 2;"
          >
            <div 
              class="card-flip-inner {isFlipped ? 'flipped' : ''} {cardAnimation === 'fly-out' ? 'card-fly-out' : ''} {cardAnimation === 'return' ? 'card-return' : ''}"
              onclick={toggleFlip}
              role="button"
              tabindex="0"
              onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && toggleFlip()}
            >
              <!-- Front Face -->
              <div 
                class="card-face neu-raised"
                style="
                  background: var(--bg-card);
                  box-shadow: var(--neu-up);
                  border-radius: var(--radius-lg);
                  padding: 52px 40px;
                  min-height: 320px;
                  display: flex;
                  flex-direction: column;
                  align-items: center;
                  justify-content: center;
                "
              >
                <div 
                  class="prose prose-lg max-w-none text-center"
                  style="font-family: var(--serif); font-size: 26px; color: var(--text-primary);"
                >
                  {@html currentCard.front}
                </div>
                <p 
                  class="mt-6"
                  style="font-family: var(--sans); font-size: 12px; color: var(--text-muted);"
                >
                  Tap to reveal
                </p>
              </div>

              <!-- Back Face -->
              <div 
                class="card-face card-back-face neu-raised"
                style="
                  background: var(--bg-card-raised);
                  box-shadow: var(--neu-up);
                  border-radius: var(--radius-lg);
                  padding: 52px 40px;
                  min-height: 320px;
                  display: flex;
                  flex-direction: column;
                  align-items: center;
                  justify-content: center;
                "
              >
                <div 
                  class="mb-4"
                  style="font-family: var(--sans); font-size: 11px; text-transform: uppercase; color: var(--text-muted); letter-spacing: 0.05em;"
                >
                  ANSWER
                </div>
                <div 
                  class="prose prose-lg max-w-none text-center"
                  style="font-family: var(--serif); font-size: 24px; color: var(--text-primary);"
                >
                  {@html currentCard.back}
                </div>
                <p 
                  class="mt-6"
                  style="font-family: var(--sans); font-size: 12px; color: var(--text-muted);"
                >
                  Tap to flip back
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- Answer Buttons (only visible when flipped) -->
        {#if isFlipped && cardAnimation === 'idle'}
          <div 
            class="flex gap-3.5 mt-8"
            style="animation: fadeUp 0.3s ease-out;"
          >
            <button
              onclick={() => answerCard(1)}
              class="flex-1 neu-raised neu-btn cursor-pointer"
              style="
                background: var(--bg-card);
                box-shadow: var(--neu-up);
                border-radius: var(--radius-md);
                padding: 16px 8px;
                border: none;
                text-align: center;
              "
            >
              <div style="font-family: var(--sans); font-size: 14px; font-weight: 600; color: var(--danger);">
                Again
              </div>
              {#if prefs.show_intervals_on_buttons}
                <div style="font-family: var(--sans); font-size: 11px; color: var(--text-muted); margin-top: 4px;">
                  {getIntervalText(1)}
                </div>
              {/if}
            </button>

            <button
              onclick={() => answerCard(2)}
              class="flex-1 neu-raised neu-btn cursor-pointer"
              style="
                background: var(--bg-card);
                box-shadow: var(--neu-up);
                border-radius: var(--radius-md);
                padding: 16px 8px;
                border: none;
                text-align: center;
              "
            >
              <div style="font-family: var(--sans); font-size: 14px; font-weight: 600; color: var(--warning);">
                Hard
              </div>
              {#if prefs.show_intervals_on_buttons}
                <div style="font-family: var(--sans); font-size: 11px; color: var(--text-muted); margin-top: 4px;">
                  {getIntervalText(2)}
                </div>
              {/if}
            </button>

            <button
              onclick={() => answerCard(3)}
              class="flex-1 neu-raised neu-btn cursor-pointer"
              style="
                background: var(--bg-card);
                box-shadow: var(--neu-up);
                border-radius: var(--radius-md);
                padding: 16px 8px;
                border: none;
                text-align: center;
              "
            >
              <div style="font-family: var(--sans); font-size: 14px; font-weight: 600; color: var(--success);">
                Good
              </div>
              {#if prefs.show_intervals_on_buttons}
                <div style="font-family: var(--sans); font-size: 11px; color: var(--text-muted); margin-top: 4px;">
                  {getIntervalText(3)}
                </div>
              {/if}
            </button>

            <button
              onclick={() => answerCard(4)}
              class="flex-1 neu-raised neu-btn cursor-pointer"
              style="
                background: var(--bg-card);
                box-shadow: var(--neu-up);
                border-radius: var(--radius-md);
                padding: 16px 8px;
                border: none;
                text-align: center;
              "
            >
              <div style="font-family: var(--sans); font-size: 14px; font-weight: 600; color: #5B9BD5;">
                Easy
              </div>
              {#if prefs.show_intervals_on_buttons}
                <div style="font-family: var(--sans); font-size: 11px; color: var(--text-muted); margin-top: 4px;">
                  {getIntervalText(4)}
                </div>
              {/if}
            </button>
          </div>
        {/if}
      {/if}
    </div>
  </main>

  <!-- Mini Deck -->
  {#if currentCard}
    <MiniDeck 
      bind:this={miniDeckRef}
      reviewedCount={reviewedCount}
      onNavigateToBrowser={navigateToBrowser}
    />
  {/if}
</div>

<style>
  .card-flip-container {
    perspective: 1200px;
  }

  .card-flip-inner {
    transition: transform 0.55s cubic-bezier(0.4, 0, 0.15, 1);
    transform-style: preserve-3d;
    position: relative;
    cursor: pointer;
  }

  .card-flip-inner.flipped {
    transform: rotateY(180deg);
  }

  .card-face {
    backface-visibility: hidden;
  }

  .card-back-face {
    position: absolute;
    inset: 0;
    transform: rotateY(180deg);
  }

  .card-fly-out {
    animation: cardFlyToDeck 0.7s cubic-bezier(0.4,0,0.2,1) forwards;
  }

  .card-return {
    animation: cardReturnShuffle 0.65s cubic-bezier(0.4,0,0.2,1);
  }

  @keyframes cardFlyToDeck {
    0% { transform: scale(1) translate(0,0) rotate(0deg); opacity:1; }
    50% { transform: scale(0.45) translate(260px,190px) rotate(12deg); opacity:0.85; }
    100% { transform: scale(0.1) translate(430px,310px) rotate(20deg); opacity:0; }
  }

  @keyframes cardReturnShuffle {
    0% { transform: translateX(0) translateY(0) rotate(0deg); opacity:1; }
    20% { transform: translateX(-60px) translateY(-10px) rotate(-8deg); opacity:0.7; }
    50% { transform: translateX(-30px) translateY(25px) rotate(4deg); opacity:0.4; }
    70% { transform: translateX(10px) translateY(10px) rotate(-2deg); opacity:0.6; }
    100% { transform: translateX(0) translateY(0) rotate(0deg); opacity:1; }
  }

  @keyframes scaleIn {
    from { opacity:0; transform:scale(0.95); }
    to { opacity:1; transform:scale(1); }
  }
</style>
