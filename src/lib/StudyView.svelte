<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { onMount, onDestroy, tick } from "svelte";
  import MiniDeck from "./MiniDeck.svelte";
  import { prefs } from "./prefs.svelte.ts";
  import { filterCardHtml } from "./pluginEngine";
  import { addToast } from "./toast";
  import { rewriteMediaUrls } from "./media";
  import { renderMath, clearMathJaxCache, preprocessAnkiMath } from "./mathjax";
  import { studyNav } from "./studyNav.svelte.ts";
  import { sendMilestoneNotification } from "./notifications";

  // Props using Svelte 5 runes
  interface Props {
    deckId: number;
    deckName: string;
    onExit: () => void;
  }

  let { deckId, deckName, onExit }: Props = $props();

  // Core state
  let isLoading = $state(true);
  let error = $state("");
  let currentCard = $state<{
    card_id: number;
    note_id: number;
    front: string;
    back: string;
    flag: number;
    again_interval: string;
    hard_interval: string;
    good_interval: string;
    easy_interval: string;
  } | null>(null);
  let isFlipped = $state(false);
  let cardAnimation = $state<'idle' | 'fly-out' | 'return'>('idle');
  let isAnswering = $state(false);
  let reviewedCount = $state(0);
  let totalCards = $state(0);
  let remainingCards = $state(0);

  // Flag state
  let currentFlag = $state(0);
  let showFlagPicker = $state(false);

  // Undo
  let canUndo = $state(false);

  // MiniDeck ref
  let miniDeckRef: MiniDeck;

  // MathJax refs
  let frontEl: HTMLElement;
  let backEl: HTMLElement;

  // Load deck stats
  async function loadDeckStats() {
    try {
      const stats = await invoke<{ new: number; review: number; learning: number }>(
        "get_deck_stats_for_review", { deckId }
      );
      totalCards = stats.new + stats.learning + stats.review;
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
      
      // Run plugin filters on card HTML
      let frontHtml = card.front;
      let backHtml = card.back;
      try {
        frontHtml = await filterCardHtml('card:render:front', frontHtml, card.card_id, card.note_id);
        backHtml = await filterCardHtml('card:render:back', backHtml, card.card_id, card.note_id);
      } catch (e) {
        console.error("Plugin filter error:", e);
      }
      
      // Rewrite media URLs for local serving
      frontHtml = rewriteMediaUrls(frontHtml);
      backHtml = rewriteMediaUrls(backHtml);
      
      // Preprocess math notation
      frontHtml = preprocessAnkiMath(frontHtml);
      backHtml = preprocessAnkiMath(backHtml);
      
      currentCard = { ...card, front: frontHtml, back: backHtml };
      currentFlag = card.flag;
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

  // Toggle flip
  function toggleFlip() {
    if (cardAnimation !== 'idle') return;
    isFlipped = !isFlipped;
  }

  // Answer card
  async function answerCard(ease: number) {
    if (!currentCard || cardAnimation !== 'idle' || isAnswering) return;
    isAnswering = true;

    const isAgain = ease === 1;
    cardAnimation = isAgain ? 'return' : 'fly-out';

    // Wait for animation
    await new Promise(resolve => setTimeout(resolve, isAgain ? 650 : 500));

    try {
      await invoke("answer_card", { cardId: currentCard.card_id, ease });
      reviewedCount++;
      remainingCards = Math.max(0, remainingCards - 1);

      // Check for milestone notifications
      if (prefs.notifications_enabled) {
        if (reviewedCount === 50) sendMilestoneNotification('reviews', 50);
        if (reviewedCount === 100) sendMilestoneNotification('reviews', 100);
        if (reviewedCount === 200) sendMilestoneNotification('reviews', 200);
      }

      // Trigger MiniDeck receive animation on success
      if (!isAgain && miniDeckRef) {
        miniDeckRef.triggerReceiveAnimation();
      }

      await loadNextCard();
      await checkUndoStatus();
    } catch (e) {
      error = String(e);
      cardAnimation = 'idle';
    } finally {
      isAnswering = false;
    }
  }

  // Get interval text for buttons
  function getIntervalText(ease: number): string {
    if (!currentCard) return "";
    switch (ease) {
      case 1: return currentCard.again_interval;
      case 2: return currentCard.hard_interval;
      case 3: return currentCard.good_interval;
      case 4: return currentCard.easy_interval;
      default: return "";
    }
  }

  // Set flag
  async function setFlag(flag: number) {
    if (!currentCard) return;
    try {
      await invoke("set_card_flag", { cardId: currentCard.card_id, flag });
      currentFlag = flag;
      showFlagPicker = false;
      studyNav.showFlagPicker = false;
      addToast(flag === 0 ? "Flag removed" : `Flag ${flag} set`, "success");
    } catch (e) {
      console.error("Error setting flag:", e);
    }
  }

  // Undo last action
  async function undoLastAction() {
    try {
      const result = await invoke<{ action_name: string; card_id: number | null }>("undo_last_action");
      addToast(result.action_name + " undone", "success");
      await loadDeckStats();
      await loadNextCard();
      await checkUndoStatus();
    } catch (e) {
      const msg = String(e);
      if (msg.includes("Nothing to undo")) {
        addToast("Nothing to undo", "warning");
      } else {
        console.error("Error undoing:", e);
      }
    }
  }

  // Check undo status
  async function checkUndoStatus() {
    try {
      const status = await invoke<{ can_undo: boolean }>("get_undo_status");
      canUndo = status.can_undo;
    } catch (e) {
      canUndo = false;
    }
  }

  // Keyboard handler
  function handleKeydown(event: KeyboardEvent) {
    if (isLoading || isAnswering) return;

    // Ctrl+Z / Cmd+Z for undo
    if ((event.ctrlKey || event.metaKey) && event.key === 'z') {
      event.preventDefault();
      undoLastAction();
      return;
    }

    // Ctrl+0-7 for flags
    if ((event.ctrlKey || event.metaKey) && /^[0-7]$/.test(event.key)) {
      event.preventDefault();
      setFlag(parseInt(event.key));
      return;
    }

    // Space to flip
    if (event.code === "Space" && !isFlipped) {
      event.preventDefault();
      toggleFlip();
      return;
    }

    // 1-4 to answer when flipped
    if (isFlipped && cardAnimation === 'idle') {
      if (event.key === "1") { event.preventDefault(); answerCard(1); }
      else if (event.key === "2") { event.preventDefault(); answerCard(2); }
      else if (event.key === "3") { event.preventDefault(); answerCard(3); }
      else if (event.key === "4") { event.preventDefault(); answerCard(4); }
    }

    // Escape to exit
    if (event.code === "Escape") onExit();
  }

  // Navigate to browser
  function navigateToBrowser() {
    if ((window as any).openCardBrowser) {
      (window as any).openCardBrowser('');
    }
  }

  // MathJax rendering via $effect
  $effect(() => {
    if (currentCard && frontEl) {
      tick().then(() => {
        clearMathJaxCache(frontEl);
        renderMath(frontEl);
      });
    }
  });

  $effect(() => {
    if (currentCard && isFlipped && backEl) {
      tick().then(() => {
        clearMathJaxCache(backEl);
        renderMath(backEl);
      });
    }
  });

  // Progress percentage
  const progressPercent = $derived(totalCards > 0 ? Math.round((reviewedCount / totalCards) * 100) : 0);

  // Sync study state → global nav store
  $effect(() => {
    if (!studyNav.active) return;
    studyNav.remainingCards = remainingCards;
    studyNav.reviewedCount = reviewedCount;
    studyNav.currentFlag = currentFlag;
    studyNav.canUndo = canUndo;
  });

  $effect(() => {
    if (!studyNav.active) return;
    studyNav.progress = progressPercent;
  });

  // Flag colors
  const flagColors = [
    'transparent',
    '#EF4444',
    '#F97316',
    '#22C55E',
    '#3B82F6',
    '#EC4899',
    '#14B8A6',
    '#8B5CF6'
  ];

  // Mount
  onMount(async () => {
    const tauriCheck = await isTauri();
    if (!tauriCheck) {
      error = "This feature requires Tauri desktop environment";
      isLoading = false;
      return;
    }
    await loadDeckStats();
    await loadNextCard();
    await checkUndoStatus();
    window.addEventListener('keydown', handleKeydown);

    // Wire study state to global navbar
    studyNav.activate(deckName, onExit);
    studyNav.setFlag = setFlag;
    studyNav.undo = undoLastAction;
  });

  onDestroy(() => {
    window.removeEventListener('keydown', handleKeydown);
    studyNav.deactivate();
  });
</script>

<div class="h-full flex flex-col overflow-hidden" style="background: var(--bg-base);">
  <!-- Progress Bar -->
  <div class="px-6 pt-4 pb-2">
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

  <!-- MAIN CONTENT -->
  <main class="flex-1 flex items-center justify-center p-6">
    <div class="w-full max-w-[680px]">

      {#if isLoading}
        <!-- Loading spinner -->
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

      {:else if error}
        <!-- Error state -->
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

      {:else if !currentCard}
        <!-- All done state -->
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
            Back to Decks
          </button>
        </div>

      {:else}
        <!-- THE CARD -->
        <div 
          class="card-flip-container relative"
          onclick={toggleFlip}
          style="cursor: pointer;"
        >
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
            class="card-flip-inner relative {isFlipped ? 'flipped' : ''} {cardAnimation === 'fly-out' ? 'card-fly-out' : ''} {cardAnimation === 'return' ? 'card-return' : ''}"
            style="z-index: 2;"
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
                class="card-content prose prose-lg max-w-none text-center"
                style="font-family: var(--serif); font-size: 26px; color: var(--text-primary);"
                bind:this={frontEl}
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
                class="card-content prose prose-lg max-w-none text-center"
                style="font-family: var(--serif); font-size: 24px; color: var(--text-primary);"
                bind:this={backEl}
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

        <!-- Answer Buttons (only visible when flipped) -->
        {#if isFlipped && cardAnimation === 'idle'}
          <div 
            class="flex gap-3.5 mt-8"
            style="animation: fadeUp 0.3s ease-out;"
          >
            <button
              onclick={() => answerCard(1)}
              disabled={isAnswering}
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
              disabled={isAnswering}
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
              disabled={isAnswering}
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
              disabled={isAnswering}
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
        {:else if !isFlipped}
          <!-- "Tap to flip" hint when not flipped -->
          <div 
            class="mt-6 text-center"
            style="font-family: var(--sans); color: var(--text-muted); font-size: 13px;"
          >
            Tap card or press Space to reveal answer
          </div>
        {/if}

      {/if}
    </div>
  </main>

  <!-- MiniDeck — only show when there's a card -->
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
    transition: transform 0.38s cubic-bezier(0.4, 0, 0.12, 1);
    transform-style: preserve-3d;
    position: relative;
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
    animation: cardFlyToDeck 0.5s cubic-bezier(0.4,0,0.2,1) forwards;
  }

  .card-return {
    animation: cardReturnShuffle 0.45s cubic-bezier(0.4,0,0.2,1);
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

  @keyframes fadeUp {
    from { opacity:0; transform:translateY(10px); }
    to { opacity:1; transform:translateY(0); }
  }

  /* Card content styling for Anki HTML */
  :global(.card-content img) {
    max-width: 100%;
    height: auto;
    border-radius: 8px;
  }

  :global(.card-content table) {
    border-collapse: collapse;
    margin: 0 auto;
  }

  :global(.card-content td),
  :global(.card-content th) {
    padding: 4px 8px;
    border: 1px solid var(--border);
  }
</style>
