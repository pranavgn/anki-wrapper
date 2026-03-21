<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { onMount, onDestroy, tick } from "svelte";
  import { fly, fade } from "svelte/transition";
  import { rewriteMediaUrls } from "./media";
  import { addToast, toast } from "./toast";
  import { renderMath, clearMathJaxCache, preprocessAnkiMath } from "./mathjax";

  // Props using Svelte 5 runes
  interface Props {
    deckId: number;
    deckName: string;
    onExit: () => void;
  }

  let { deckId, deckName, onExit }: Props = $props();

  // Card state machine
  type CardState = 'idle' | 'flipped' | 'leaving-left' | 'leaving-right' | 'leaving-up' | 'leaving-down' | 'entering';
  let cardState: CardState = $state('idle');

  // State
  let isLoading = $state(true);
  let error = $state("");
  let currentCard: {
    card_id: number;
    front: string;
    back: string;
    note_id: number;
    flag: number;
    again_interval: string;
    hard_interval: string;
    good_interval: string;
    easy_interval: string;
  } | null = $state(null);
  let currentTags = $state<string[]>([]);
  let isAnswerRevealed = $state(false);
  let isAnswering = $state(false);
  let reviewedToday = $state(0);
  let totalDueToday = $state(0);
  let remainingCards = $state(0);
  let canUndo = $state(false);
  
  // Flag state
  let currentFlag = $state(0);
  let showFlagPicker = $state(false);

  // Load first card on mount
  onMount(async () => {
    const tauriCheck = await isTauri();
    if (!tauriCheck) {
      error = "This feature requires Tauri desktop environment";
      isLoading = false;
      return;
    }
    
    await loadDeckStats();
    loadNextCard();
    await checkUndoStatus();
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });

  async function loadDeckStats() {
    try {
      const stats = await invoke<{ new: number; learning: number; review: number }>(
        "get_deck_stats_for_review",
        { deck_id: deckId }
      );
      totalDueToday = stats.new + stats.learning + stats.review;
      remainingCards = totalDueToday;
    } catch (e: unknown) {
      console.error("Error loading deck stats:", e);
      totalDueToday = 0;
      remainingCards = 0;
    }
  }

  async function checkUndoStatus() {
    try {
      const status = await invoke<{ can_undo: boolean; undo_label: string | null; can_redo: boolean; redo_label: string | null }>(
        "get_undo_status"
      );
      canUndo = status.can_undo;
    } catch (e: unknown) {
      console.error("Error checking undo status:", e);
      canUndo = false;
    }
  }

  async function undoLastAction() {
    try {
      const result = await invoke<{ action_name: string; card_id: number | null }>(
        "undo_last_action"
      );
      addToast(result.action_name + " undone", "success");
      // Reload the current card state
      await loadDeckStats();
      await loadNextCard();
      await checkUndoStatus();
    } catch (e: unknown) {
      const errorMsg = String(e);
      if (errorMsg.includes("Nothing to undo")) {
        addToast("Nothing to undo", "error");
      } else {
        console.error("Error undoing action:", e);
      }
    }
  }

  async function loadNextCard() {
    isLoading = true;
    error = "";
    
    try {
      const card = await invoke<{
        card_id: number;
        front: string;
        back: string;
        note_id: number;
        flag: number;
        again_interval: string;
        hard_interval: string;
        good_interval: string;
        easy_interval: string;
      }>("get_next_card", { deck_id: deckId });
      
      currentCard = card;
      isAnswerRevealed = false;
      cardState = 'entering';
      remainingCards = Math.max(0, remainingCards - 1);
      
      // Load note tags
      try {
        currentTags = await invoke<string[]>("get_note_tags", { note_id: card.note_id });
      } catch (e) {
        console.error("Error loading note tags:", e);
        currentTags = [];
      }
      
      // Set current flag
      currentFlag = card.flag || 0;
      showFlagPicker = false;
      
      // Animate card entrance
      setTimeout(() => {
        if (cardState === 'entering') {
          cardState = 'idle';
        }
      }, 240);
    } catch (e: unknown) {
      error = String(e);
      if (error.includes("No cards left")) {
        currentCard = null;
        remainingCards = 0;
        cardState = 'idle';
      }
    } finally {
      isLoading = false;
    }
  }

  function revealAnswer() {
    isAnswerRevealed = true;
    cardState = 'flipped';
  }

  async function answerCard(ease: number) {
    if (!currentCard || isAnswering || cardState !== 'flipped') return;
    
    isAnswering = true;
    
    // Trigger leaving animation based on ease
    switch(ease) {
      case 1: cardState = 'leaving-left'; break;  // Again
      case 2: cardState = 'leaving-down'; break;  // Hard
      case 3: cardState = 'leaving-up'; break;    // Good
      case 4: cardState = 'leaving-right'; break; // Easy
    }
    
    try {
      const result = await invoke<{ card_id: number; leech: boolean; suspended: boolean }>("answer_card", { 
        card_id: currentCard.card_id, 
        ease 
      });
      
      // Show warning toast if card was marked as leech
      if (result.leech) {
        const cardId = currentCard.card_id;
        toast.warning(
          "Card marked as leech and suspended — you've missed this card too many times. Consider rewriting it. " +
          "<button class='underline font-bold ml-2' onclick=\"window.openCardBrowser && window.openCardBrowser('cid:" + cardId + "')\">View Card</button>"
        );
      }
      
      reviewedToday++;
      
      // Wait for leaving animation then load next card
      setTimeout(async () => {
        await loadNextCard();
        await checkUndoStatus();
      }, 280);
    } catch (e: unknown) {
      error = String(e);
      cardState = 'idle';
    } finally {
      isAnswering = false;
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (isLoading || isAnswering) return;
    
    // Ctrl+Z or Cmd+Z for undo
    if ((event.ctrlKey || event.metaKey) && event.key === 'z') {
      event.preventDefault();
      undoLastAction();
      return;
    }
    
    // Flag shortcuts Ctrl+0 through Ctrl+7
    if ((event.ctrlKey || event.metaKey) && ['0', '1', '2', '3', '4', '5', '6', '7'].includes(event.key)) {
      event.preventDefault();
      setFlag(parseInt(event.key));
      return;
    }
    
    if (event.code === "Space" && !isAnswerRevealed) {
      event.preventDefault();
      revealAnswer();
      return;
    }
    
    if (isAnswerRevealed && cardState === 'flipped') {
      if (event.key === "1") {
        event.preventDefault();
        answerCard(1);
      } else if (event.key === "2") {
        event.preventDefault();
        answerCard(2);
      } else if (event.key === "3") {
        event.preventDefault();
        answerCard(3);
      } else if (event.key === "4") {
        event.preventDefault();
        answerCard(4);
      }
    }
    
    if (event.code === "Escape") {
      onExit();
    }
  }

  // Refs for math rendering
  let frontEl: HTMLElement;
  let backEl: HTMLElement;
  let prevFrontHtml = $state('');
  let prevBackHtml = $state('');

  let frontHtml = $derived.by(() => {
    if (currentCard) return preprocessAnkiMath(rewriteMediaUrls(currentCard.front));
    return "";
  });
  let backHtml = $derived.by(() => {
    if (currentCard) return preprocessAnkiMath(rewriteMediaUrls(currentCard.back));
    return "";
  });

  // Render math when front content changes
  $effect(() => {
    if (frontHtml && frontEl && frontHtml !== prevFrontHtml) {
      if (prevFrontHtml) {
        clearMathJaxCache(frontEl);
      }
      prevFrontHtml = frontHtml;
      tick().then(() => renderMath(frontEl));
    }
  });

  // Render math when back content changes
  $effect(() => {
    if (backHtml && backEl && backHtml !== prevBackHtml) {
      if (prevBackHtml) {
        clearMathJaxCache(backEl);
      }
      prevBackHtml = backHtml;
      tick().then(() => renderMath(backEl));
    }
  });
  let progress = $derived(totalDueToday > 0 ? (reviewedToday / totalDueToday) * 100 : 0);
  
  // Flag color mapping
  const FLAG_COLORS: Record<number, string> = {
    0: 'transparent',
    1: '#EF4444', // red
    2: '#F97316', // orange
    3: '#22C55E', // green
    4: '#3B82F6', // blue
    5: '#EC4899', // pink
    6: '#14B8A6', // turquoise
    7: '#8B5CF6', // purple
  };
  
  let flagBorderColor = $derived(FLAG_COLORS[currentFlag] || 'transparent');
  
  // Click outside action
  function clickOutside(node: HTMLElement) {
    const handleClick = (event: MouseEvent) => {
      if (node && !node.contains(event.target as Node) && !event.defaultPrevented) {
        showFlagPicker = false;
      }
    };
    document.addEventListener('click', handleClick);
    return {
      destroy() {
        document.removeEventListener('click', handleClick);
      }
    };
  }
  
  // Set flag function
  async function setFlag(flag: number) {
    if (!currentCard) return;
    
    const previousFlag = currentFlag;
    currentFlag = flag;
    showFlagPicker = false;
    
    try {
      await invoke("set_card_flag", {
        card_id: currentCard.card_id,
        flag: flag
      });
    } catch (e) {
      // Revert on error
      currentFlag = previousFlag;
      addToast(e instanceof Error ? e.message : "Failed to set flag", "error");
    }
  }
</script>

<div class="fixed inset-0 bg-bg-base flex flex-col overflow-hidden">
  <!-- Progress Bar -->
  <div class="absolute top-0 left-0 right-0 h-1.5 bg-bg-subtle">
    <div
      class="h-full bg-accent transition-all duration-400 ease-out"
      style={`width: ${progress}%`}
    ></div>
  </div>

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
      
      <button
        onclick={undoLastAction}
        disabled={!canUndo}
        class="flex items-center gap-1 text-text-secondary hover:text-text-primary transition-colors disabled:opacity-40 disabled:cursor-not-allowed ml-2"
        aria-label="Undo last action"
        title={canUndo ? "Undo last action (Ctrl+Z)" : "Nothing to undo"}
      >
        <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
        </svg>
      </button>
      
      <!-- Flag Button -->
      <div class="relative ml-2">
        <button
          onclick={() => showFlagPicker = !showFlagPicker}
          class="flex items-center justify-center w-8 h-8 rounded-lg hover:bg-bg-subtle transition-colors"
          aria-label="Set flag"
          title="Flag card (⌘1-6)"
        >
          <svg 
            class="h-5 w-5" 
            fill={currentFlag > 0 ? FLAG_COLORS[currentFlag] : "none"} 
            stroke={currentFlag > 0 ? FLAG_COLORS[currentFlag] : "rgba(148, 163, 184, 0.35)"} 
            stroke-width="2" 
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" d="M3 21v-4m0 0V5a2 2 0 012-2h6.5l1 1H21l-3 6 3 6h-8.5l-1-1H5a2 2 0 00-2 2zm9-13.5V9" />
          </svg>
        </button>
        
        <!-- Flag Picker Popover -->
        {#if showFlagPicker}
          <div 
            class="absolute top-full right-0 mt-1 z-50 bg-white rounded-2xl shadow-lg border border-border p-2 gap-1.5 flex flex-row"
            use:clickOutside
          >
            <button
              onclick={() => setFlag(0)}
              class="w-7 h-7 rounded-full bg-gray-300 hover:ring-2 hover:ring-offset-1 hover:ring-gray-400 flex items-center justify-center"
              title="Remove flag"
            >
              <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
            <button
              onclick={() => setFlag(1)}
              class="w-7 h-7 rounded-full bg-[#EF4444] hover:ring-2 hover:ring-offset-1 hover:ring-red-400"
              title="Red flag"
            ></button>
            <button
              onclick={() => setFlag(2)}
              class="w-7 h-7 rounded-full bg-[#F97316] hover:ring-2 hover:ring-offset-1 hover:ring-orange-400"
              title="Orange flag"
            ></button>
            <button
              onclick={() => setFlag(3)}
              class="w-7 h-7 rounded-full bg-[#22C55E] hover:ring-2 hover:ring-offset-1 hover:ring-green-400"
              title="Green flag"
            ></button>
            <button
              onclick={() => setFlag(4)}
              class="w-7 h-7 rounded-full bg-[#3B82F6] hover:ring-2 hover:ring-offset-1 hover:ring-blue-400"
              title="Blue flag"
            ></button>
            <button
              onclick={() => setFlag(5)}
              class="w-7 h-7 rounded-full bg-[#EC4899] hover:ring-2 hover:ring-offset-1 hover:ring-pink-400"
              title="Pink flag"
            ></button>
            <button
              onclick={() => setFlag(6)}
              class="w-7 h-7 rounded-full bg-[#14B8A6] hover:ring-2 hover:ring-offset-1 hover:ring-teal-400"
              title="Turquoise flag"
            ></button>
          </div>
        {/if}
      </div>
    </div>
    
    {#if remainingCards > 0}
      <div class="text-sm text-text-secondary">
        {remainingCards} remaining
      </div>
    {/if}
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
      {:else if error && !error.includes("No cards left")}
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
            You've reviewed {reviewedToday} cards today. Great job!
          </p>
          <button
            onclick={onExit}
            class="px-6 py-3 bg-accent text-white rounded-xl hover:bg-accent/90 transition-colors font-medium cursor-pointer active:scale-95"
          >
            Back to Decks
          </button>
        </div>
      
      <!-- Card Display with 3D Stack -->
      {:else if currentCard}
        <div class="relative" style="perspective: 1200px; min-height: 400px;">
          <!-- Ghost Card 2 (bottom) -->
          <div 
            class="absolute inset-0 bg-bg-card/40 rounded-3xl border border-border/50"
            style="transform: translateY(12px) scale(0.94); z-index: 0;"
          ></div>
          
          <!-- Ghost Card 1 (middle) -->
          <div 
            class="absolute inset-0 bg-bg-card/60 rounded-3xl border border-border/70"
            style="transform: translateY(6px) scale(0.97); z-index: 1;"
          ></div>
          
          <!-- Active Card with 3D Flip -->
          <div 
            class="bg-bg-card rounded-3xl shadow-warm border border-border overflow-hidden relative"
            style="transform-style: preserve-3d; z-index: 2; {cardState === 'flipped' ? 'transform: rotateY(180deg);' : ''} {cardState === 'leaving-left' ? 'transform: translateX(-120%) rotate(-8deg);' : ''} {cardState === 'leaving-right' ? 'transform: translateX(120%) rotate(8deg);' : ''} {cardState === 'leaving-up' ? 'transform: translateY(-60px); opacity: 0;' : ''} {cardState === 'leaving-down' ? 'transform: translateY(60px); opacity: 0;' : ''} {cardState === 'entering' ? 'transform: scale(0.93); opacity: 0;' : ''} transition: {cardState === 'flipped' ? 'transform 480ms cubic-bezier(0.4, 0, 0.2, 1)' : ''}, {cardState.startsWith('leaving') ? 'transform 280ms ease-in, opacity 280ms ease-in' : ''}, {cardState === 'entering' ? 'transform 240ms ease-out, opacity 240ms ease-out' : ''}; border-top: 3px solid {flagBorderColor}"
          >
            <!-- Front Face -->
            <div 
              class="p-10 min-h-[320px] flex items-center justify-center"
              style="backface-visibility: hidden; transform: rotateY(0deg);"
            >
              <div class="text-center card-content">
                <p class="text-1.5xl text-text-primary leading-relaxed" bind:this={frontEl}>
                  {@html frontHtml}
                </p>
              </div>
            </div>
            
            <!-- Back Face -->
            <div 
              class="absolute inset-0 p-10 min-h-[320px] flex flex-col items-center justify-center bg-bg-card"
              style="backface-visibility: hidden; transform: rotateY(180deg);"
            >
              <div class="text-center card-content flex-1 flex items-center justify-center">
                <p class="text-xl text-text-primary leading-relaxed" bind:this={backEl}>
                  {@html backHtml}
                </p>
              </div>
              
              <!-- Tags displayed when answer is revealed -->
              {#if isAnswerRevealed && currentTags.length > 0}
                <div class="mt-4 flex flex-wrap gap-1 justify-center">
                  {#each currentTags as tag}
                    <span class="bg-bg-subtle text-text-secondary text-xs rounded-full px-2 py-0.5">
                      {tag}
                    </span>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        </div>

        <!-- Actions -->
        <div class="mt-8 p-8">
          {#if !isAnswerRevealed}
            <button
              onclick={revealAnswer}
              disabled={isLoading || isAnswering}
              class="w-full py-4 bg-bg-base hover:bg-bg-base/80 disabled:bg-bg-subtle/50 text-text-primary rounded-2xl transition-colors text-lg font-medium cursor-pointer active:scale-95"
            >
              Show Answer
            </button>
          {:else}
            <div class="grid grid-cols-4 gap-3">
              <button
                onclick={() => answerCard(1)}
                disabled={isAnswering}
                class="py-3 bg-[#FEE2E2] hover:bg-[#FECACA] disabled:bg-bg-subtle/50 text-[#991B1B] rounded-xl transition-all hover:-translate-y-0.5 text-sm font-medium cursor-pointer active:scale-95"
                style="animation-delay: 0ms"
              >
                <div>Again</div>
                {#if currentCard?.again_interval}
                  <div class="text-xs opacity-75">{currentCard.again_interval}</div>
                {/if}
              </button>
              <button
                onclick={() => answerCard(2)}
                disabled={isAnswering}
                class="py-3 bg-[#FEF3C7] hover:bg-[#FDE68A] disabled:bg-bg-subtle/50 text-[#92400E] rounded-xl transition-all hover:-translate-y-0.5 text-sm font-medium cursor-pointer active:scale-95"
                style="animation-delay: 60ms"
              >
                <div>Hard</div>
                {#if currentCard?.hard_interval}
                  <div class="text-xs opacity-75">{currentCard.hard_interval}</div>
                {/if}
              </button>
              <button
                onclick={() => answerCard(3)}
                disabled={isAnswering}
                class="py-3 bg-[#DBEAFE] hover:bg-[#BFDBFE] disabled:bg-bg-subtle/50 text-[#1E40AF] rounded-xl transition-all hover:-translate-y-0.5 text-sm font-medium cursor-pointer active:scale-95"
                style="animation-delay: 120ms"
              >
                <div>Good</div>
                {#if currentCard?.good_interval}
                  <div class="text-xs opacity-75">{currentCard.good_interval}</div>
                {/if}
              </button>
              <button
                onclick={() => answerCard(4)}
                disabled={isAnswering}
                class="py-3 bg-[#D1FAE5] hover:bg-[#A7F3D0] disabled:bg-bg-subtle/50 text-[#065F46] rounded-xl transition-all hover:-translate-y-0.5 text-sm font-medium cursor-pointer active:scale-95"
                style="animation-delay: 180ms"
              >
                <div>Easy</div>
                {#if currentCard?.easy_interval}
                  <div class="text-xs opacity-75">{currentCard.easy_interval}</div>
                {/if}
              </button>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </main>

  <!-- Keyboard Hints -->
  {#if !isLoading && currentCard}
    <footer class="relative z-10 px-6 py-4 text-sm text-text-secondary">
      <div class="flex justify-center gap-6">
        {#if !isAnswerRevealed}
          <span><kbd class="px-2 py-1 bg-bg-base rounded text-xs">Space</kbd> Show Answer</span>
        {:else}
          <span><kbd class="px-2 py-1 bg-bg-base rounded text-xs">1</kbd> Again</span>
          <span><kbd class="px-2 py-1 bg-bg-base rounded text-xs">2</kbd> Hard</span>
          <span><kbd class="px-2 py-1 bg-bg-base rounded text-xs">3</kbd> Good</span>
          <span><kbd class="px-2 py-1 bg-bg-base rounded text-xs">4</kbd> Easy</span>
        {/if}
        <span><kbd class="px-2 py-1 bg-bg-base rounded text-xs">Esc</kbd> Exit</span>
      </div>
    </footer>
  {/if}
</div>

<style>
  .card-content :global(img) {
    max-width: 100%;
    height: auto;
  }

  .card-content :global(audio) {
    width: 100%;
  }

  .card-content :global(.cloze) {
    font-weight: 600;
    color: var(--accent);
  }

  .card-content :global(.cloze-deleted) {
    background-color: var(--accent-soft);
    padding: 0.125rem 0.375rem;
    border-radius: 0.25rem;
  }
</style>
