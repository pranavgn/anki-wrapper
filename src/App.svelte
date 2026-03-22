<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { fade, fly } from "svelte/transition";
  import StudyView from "./lib/StudyView.svelte";
  import CardEditor from "./lib/CardEditor.svelte";
  import Dashboard from "./lib/Dashboard.svelte";
  import DeckOverview from "./lib/DeckOverview.svelte";
  import StatsView from "./lib/StatsView.svelte";
  import NeuToast from "./lib/ui/NeuToast.svelte";
  import KeyboardShortcuts from "./lib/KeyboardShortcuts.svelte";
  import ImportModal from "./lib/ImportModal.svelte";
  import { exportCollectionColpkg, ImportError } from "./lib/importer";
  import { prefs } from "./lib/prefs.svelte.ts";
  import { addToast } from "./lib/toast";
  import { fly_if_enabled } from "./lib/animate.svelte.ts";
  import CardBrowser from "./lib/CardBrowser.svelte";
  import NotetypeManager from "./lib/NotetypeManager.svelte";
  import Settings from "./lib/Settings.svelte";
  import ImageOcclusion from "./lib/ImageOcclusion.svelte";
  import { pluginEngine, setCurrentLoadingPlugin, clearCurrentLoadingPlugin } from "./lib/pluginEngine";
  import { loadAllPlugins } from "./lib/pluginLoader";
  import { studyNav } from "./lib/studyNav.svelte.ts";

  import PluginManager from "./lib/PluginManager.svelte";

  // Page state
  type Page = 'dashboard' | 'deckOverview' | 'study' | 'editor' | 'stats' | 'browser';
  let currentPage: Page = $state('dashboard');
  let previousPage: Page = $state('dashboard');
  let browserQuery = $state('');
  
  // Card editor state
  let editingCard = $state<any>(null);
  
  // Handle edit card from detail panel
  function handleEditCard(card: any) {
    editingCard = card;
    navigate('editor');
  }

  // Navigation function
  function navigate(page: Page) {
    previousPage = currentPage;
    currentPage = page;
  }

  // Function to open browser with a specific query (e.g., from leech toast)
  function openBrowserWithQuery(query: string) {
    browserQuery = query;
    navigate('browser');
  }

  // Keyboard shortcuts
  let showKeyboardShortcuts = $state(false);

  // Browser detection state
  let isRunningInBrowser = $state(false);
  let browserCheckComplete = $state(false);

  // Collection status for progressive loading
  type CollectionStatus = 'loading' | 'ready' | 'error';
  let collectionStatus: CollectionStatus = $state('loading');
  let collectionError: string = $state('');

  // Import modal state
  let showImportModal = $state(false);
  
  // Settings panel state
  let showSettings = $state(false);
  let showImageOcclusion = $state(false);
  
  // Notetype manager state
  let showNotetypeManager = $state(false);

  // Plugin manager state
  let showPluginManager = $state(false);

  async function handleExportCollection() {
    try {
      await exportCollectionColpkg();
      addToast("Collection exported successfully", "success");
    } catch (e) {
      if (e instanceof ImportError && e.isCancelled) return;
      addToast(e instanceof Error ? e.message : "Export failed", "error");
    }
  }

  let isCollectionOpen = $state(false);

  // Study view state
  let currentDeckId: number | null = $state(null);
  let currentDeckName = $state("");
  let activeDeck: any = $state(null);

  // Initialize on mount
  onMount(async () => {
    // Expose function to open browser with query
    (window as any).openCardBrowser = (query: string) => {
      openBrowserWithQuery(query);
    };
    
    const tauriCheck = await isTauri();
    if (!tauriCheck) {
      isRunningInBrowser = true;
      browserCheckComplete = true;
      collectionStatus = 'error';
      collectionError = 'Tauri desktop environment required';
      return;
    }
    
    browserCheckComplete = true;
    
    // Load prefs (non-critical)
    try {
      await prefs.load();
    } catch (e) {
      console.error("Non-critical init error:", e);
    }

    // Initialize collection — this is the critical path
    try {
      await invoke("init_standalone_collection");
      collectionStatus = 'ready';
      isCollectionOpen = true;

      // Dev-only: seed test deck if env flag is set
      if (import.meta.env.DEV) {
        try {
          const decks = await invoke<Array<{id: number; name: string}>>("get_all_decks");
          const hasTestDeck = decks.some(d => d.name === "🧪 Test Deck (Dev)");
          if (!hasTestDeck) {
            // Create test deck
            const deckId = await invoke<number>("create_deck", { name: "🧪 Test Deck (Dev)" });
            // Seed with sample cards across difficulty levels
            const testCards = [
              { front: "What is spaced repetition?", back: "A learning technique that incorporates increasing intervals of time between subsequent review of previously learned material." },
              { front: "What does FSRS stand for?", back: "Free Spaced Repetition Scheduler — an open-source, modern algorithm for scheduling flashcard reviews." },
              { front: "What is the forgetting curve?", back: "A mathematical model showing how information is lost over time when there is no attempt to retain it. First described by Hermann Ebbinghaus in 1885." },
              { front: "What is active recall?", back: "A principle of efficient learning that involves actively stimulating memory during the learning process, rather than passively reviewing material." },
              { front: "What is the minimum information principle?", back: "The idea that flashcards should be as simple and atomic as possible — each card should test exactly one piece of knowledge." },
              { front: "What is interleaving?", back: "A learning strategy where you mix different topics or types of problems during study, rather than focusing on one topic at a time (blocking)." },
              { front: "Who created Anki?", back: "Damien Elmes. Anki was first released in 2006 and is open source." },
              { front: "What is a leech in SRS?", back: "A card that has been failed many times (typically 8+ lapses). Leeches are automatically suspended to prevent wasting time on poorly-formed cards." },
              { front: "What is the spacing effect?", back: "The phenomenon where learning is more effective when study sessions are spaced out over time rather than concentrated in a single session (massed practice)." },
              { front: "What is retrieval practice?", back: "A learning strategy that involves recalling information from memory rather than simply re-reading it. Testing yourself strengthens long-term retention more than passive review." },
              { front: "What does 'desired retention' mean in FSRS?", back: "The target probability (e.g., 0.9 = 90%) that you will remember a card when it comes up for review. Higher retention means shorter intervals and more reviews." },
              { front: "What is the difference between recognition and recall?", back: "Recognition is identifying previously learned information when presented with it (e.g., multiple choice). Recall is retrieving information from memory without cues (e.g., flashcards). Recall produces stronger learning." },
              { front: "What is the testing effect?", back: "The finding that taking a test on material produces better long-term retention than spending the same amount of time restudying. Also known as retrieval practice effect." },
            ];
            for (const card of testCards) {
              await invoke("add_basic_card", { deckId: deckId, front: card.front, back: card.back, tags: [] });
            }
            console.log("🧪 Test deck seeded with", testCards.length, "cards");
            window.dispatchEvent(new CustomEvent('refresh-decks')); // Refresh dashboard
          } else {
            // Check if existing deck needs cards
            const testDeck = decks.find(d => d.name === "🧪 Test Deck (Dev)");
            if (testDeck) {
              const cards = await invoke<any[]>("search_cards", {
                query: `deck:"🧪 Test Deck (Dev)"`,
                order: "cardDue",
                limit: 1
              });
              if (cards.length === 0) {
                // Re-seed if deck exists but is empty
                const deckId = testDeck.id;
                const testCards = [
                  { front: "What is spaced repetition?", back: "A learning technique that incorporates increasing intervals of time between subsequent review of previously learned material." },
                  { front: "What does FSRS stand for?", back: "Free Spaced Repetition Scheduler — an open-source, modern algorithm for scheduling flashcard reviews." },
                  { front: "What is the forgetting curve?", back: "A mathematical model showing how information is lost over time when there is no attempt to retain it. First described by Hermann Ebbinghaus in 1885." },
                  { front: "What is active recall?", back: "A principle of efficient learning that involves actively stimulating memory during the learning process, rather than passively reviewing material." },
                  { front: "What is the minimum information principle?", back: "The idea that flashcards should be as simple and atomic as possible — each card should test exactly one piece of knowledge." },
                  { front: "What is interleaving?", back: "A learning strategy where you mix different topics or types of problems during study, rather than focusing on one topic at a time (blocking)." },
                  { front: "Who created Anki?", back: "Damien Elmes. Anki was first released in 2006 and is open source." },
                  { front: "What is a leech in SRS?", back: "A card that has been failed many times (typically 8+ lapses). Leeches are automatically suspended to prevent wasting time on poorly-formed cards." },
                  { front: "What is the spacing effect?", back: "The phenomenon where learning is more effective when study sessions are spaced out over time rather than concentrated in a single session (massed practice)." },
                  { front: "What is retrieval practice?", back: "A learning strategy that involves recalling information from memory rather than simply re-reading it. Testing yourself strengthens long-term retention more than passive review." },
                  { front: "What does 'desired retention' mean in FSRS?", back: "The target probability (e.g., 0.9 = 90%) that you will remember a card when it comes up for review. Higher retention means shorter intervals and more reviews." },
                  { front: "What is the difference between recognition and recall?", back: "Recognition is identifying previously learned information when presented with it (e.g., multiple choice). Recall is retrieving information from memory without cues (e.g., flashcards). Recall produces stronger learning." },
                  { front: "What is the testing effect?", back: "The finding that taking a test on material produces better long-term retention than spending the same amount of time restudying. Also known as retrieval practice effect." },
                ];
                for (const card of testCards) {
                  await invoke("add_basic_card", { deckId: deckId, front: card.front, back: card.back, tags: [] });
                }
                console.log("🧪 Test deck re-seeded with", testCards.length, "cards (deck was empty)");
                window.dispatchEvent(new CustomEvent('refresh-decks')); // Refresh dashboard
              }
            }
          }
        } catch (e) {
          console.warn("Test deck seeding failed (non-critical):", e);
        }
      }

      // Load all plugins
      await loadAllPlugins();
      
      // Fire the app:ready hook now that plugins are loaded
      await pluginEngine.runAction('app:ready', {});
    } catch (error) {
      collectionStatus = 'error';
      collectionError = error instanceof Error ? error.message : String(error);
    }
  });

  function startReview(deckId: number, deckName: string) {
    console.log("startReview called with deckId:", deckId, "deckName:", deckName);
    currentDeckId = deckId;
    currentDeckName = deckName;
    navigate('study');
    console.log("State after startReview: currentPage=", currentPage, ", currentDeckId=", currentDeckId);
  }

  function openDeckOverview(deckId: number, deckName: string) {
    activeDeck = { id: deckId, name: deckName };
    currentDeckId = deckId;
    currentDeckName = deckName;
    navigate('deckOverview');
  }

  function exitReviewMode() {
    if (currentPage === 'study' && previousPage === 'deckOverview') {
      navigate('deckOverview');
    } else {
      navigate('dashboard');
      currentDeckId = null;
      currentDeckName = "";
      activeDeck = null;
    }
  }

  function goToDashboard() {
    navigate('dashboard');
    currentDeckId = null;
    currentDeckName = "";
    activeDeck = null;
  }

  // Global keyboard shortcuts
  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === "?") {
      event.preventDefault();
      showKeyboardShortcuts = !showKeyboardShortcuts;
    }
  }
</script>

<svelte:window on:keydown={handleGlobalKeydown} onclick={() => { if (studyNav.showFlagPicker) studyNav.showFlagPicker = false; }} />

<!-- Browser Error Message -->
{#if browserCheckComplete && isRunningInBrowser}
  <main class="min-h-screen bg-bg-base p-8">
    <div class="max-w-2xl mx-auto">
      <div class="bg-white rounded-2xl shadow-warm p-6 border border-border">
        <div class="flex items-start gap-4">
          <div class="flex-shrink-0">
            <svg class="h-8 w-8 text-danger" viewBox="0 0 20 20" fill="currentColor">
              <path fill-rule="evenodd" d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
            </svg>
          </div>
          <div>
            <h2 class="text-xl font-semibold text-text-primary mb-2">Desktop App Required</h2>
            <p class="text-text-secondary mb-4">
              This application requires Tauri desktop environment to function properly. 
              The Tauri APIs (invoke) are not available in a web browser.
            </p>
            <p class="text-text-secondary text-sm mb-4">
              <strong>How to run:</strong>
            </p>
            <ul class="list-disc list-inside text-text-secondary text-sm mb-4 space-y-1">
              <li>Run <code class="bg-bg-subtle px-2 py-0.5 rounded">npm run tauri:dev</code> to start in development mode</li>
              <li>Or run <code class="bg-bg-subtle px-2 py-0.5 rounded">npm run tauri:build</code> to build the desktop app</li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  </main>
{:else}
  <!-- App Shell -->
  <div class="min-h-screen bg-bg-base flex flex-col">
    <!-- Top Navigation -->
    <nav class="flex items-center justify-between px-6 py-3" style="background: var(--bg-card); border-bottom: 1px solid var(--border); position: relative; z-index: 30;">

      <!-- ═══ LEFT SECTION ═══ -->
      <div class="flex items-center gap-2 min-w-[180px]">
        {#if currentPage === 'dashboard'}
          <!-- Logo -->
          <svg class="h-7 w-7" viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg" style="flex-shrink: 0;">
            <rect width="100" height="100" rx="22" fill="var(--accent)" />
            <path d="M30 70V32l20 24 20-24v38" stroke="white" stroke-width="7" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
          </svg>
          <span style="font-family: var(--serif); font-size: 22px; font-weight: 600; color: var(--text-primary); letter-spacing: -0.02em;">Mnemora</span>

        {:else if studyNav.active}
          <!-- Study mode: Back + Undo -->
          <button
            onclick={() => studyNav.exit?.()}
            class="neu-subtle neu-btn flex items-center gap-2 px-3 py-1.5 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
            </svg>
            <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">Back</span>
          </button>
          {#if studyNav.canUndo}
            <button
              onclick={() => studyNav.undo?.()}
              class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
              style="background: var(--bg-card);"
              title="Undo (Ctrl+Z)"
            >
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h10a8 8 0 018 8v2M3 10l6 6m-6-6l6-6" />
              </svg>
            </button>
          {/if}

        {:else}
          <!-- All other pages: Back to dashboard -->
          <button
            onclick={goToDashboard}
            class="flex items-center gap-2 cursor-pointer"
            style="background: none; border: none; padding: 0;"
          >
            <svg class="h-7 w-7" viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg" style="flex-shrink: 0;">
              <rect width="100" height="100" rx="22" fill="var(--accent)" />
              <path d="M30 70V32l20 24 20-24v38" stroke="white" stroke-width="7" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
            </svg>
            <span style="font-family: var(--serif); font-size: 20px; font-weight: 600; color: var(--text-primary); letter-spacing: -0.02em;">Mnemora</span>
          </button>
        {/if}
      </div>

      <!-- ═══ CENTER SECTION ═══ -->
      <div class="flex items-center gap-2">
        {#if currentPage === 'deckOverview' && activeDeck}
          <!-- Breadcrumb: Decks > DeckName -->
          <button onclick={goToDashboard} class="cursor-pointer" style="font-family: var(--sans); font-size: 13px; color: var(--text-muted); background: none; border: none;">
            Decks
          </button>
          <svg class="h-3 w-3" style="color: var(--text-muted);" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
          <span style="font-family: var(--sans); font-size: 13px; color: var(--text-primary); font-weight: 500;">
            {activeDeck.name}
          </span>

        {:else if studyNav.active}
          <!-- Study mode: Deck name + progress -->
          <span style="font-family: var(--serif); font-size: 17px; font-weight: 500; color: var(--text-primary);">
            {studyNav.deckName}
          </span>
          {#if studyNav.progress > 0}
            <div style="width: 60px; height: 4px; border-radius: 2px; background: var(--bg-subtle); margin-left: 8px; overflow: hidden;">
              <div style="width: {studyNav.progress}%; height: 100%; background: var(--accent); border-radius: 2px; transition: width 0.3s ease;"></div>
            </div>
          {/if}

        {:else if currentPage === 'editor'}
          <span style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
            {editingCard ? 'Edit Card' : 'Add Card'}
          </span>

        {:else if currentPage === 'stats'}
          <span style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
            Statistics
          </span>

        {:else if currentPage === 'browser'}
          <span style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
            Browse
          </span>
        {/if}
      </div>

      <!-- ═══ RIGHT SECTION ═══ -->
      <div class="flex items-center justify-end gap-2.5 min-w-[180px]">
        {#if studyNav.active}
          <!-- Study mode: Flag + Counts -->
          <div class="relative">
            <button
              onclick={() => { studyNav.showFlagPicker = !studyNav.showFlagPicker; }}
              class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
              style="background: var(--bg-card);"
              title="Set flag (Ctrl+0-7)"
            >
              <svg class="h-4 w-4" fill="{studyNav.currentFlag > 0 ? studyNav.FLAG_COLORS[studyNav.currentFlag] : 'none'}" stroke="{studyNav.currentFlag > 0 ? studyNav.FLAG_COLORS[studyNav.currentFlag] : 'var(--text-muted)'}" viewBox="0 0 24 24" stroke-width="2">
                <path stroke-linecap="round" stroke-linejoin="round" d="M3 21v-4m0 0V5a2 2 0 012-2h6.5l1 1H21l-3 6 3 6h-8.5l-1-1H5a2 2 0 00-2 2zm9-13.5V9" />
              </svg>
            </button>

            <!-- Flag picker popover -->
            {#if studyNav.showFlagPicker}
              <div
                class="absolute right-0 top-full mt-2 p-2 rounded-xl z-50 flex gap-1"
                style="background: var(--bg-card); box-shadow: 0 8px 24px rgba(0,0,0,0.12); border: 1px solid var(--border);"
              >
                {#each [0,1,2,3,4,5,6,7] as flagIdx}
                  <button
                    onclick={() => { studyNav.setFlag?.(flagIdx); studyNav.showFlagPicker = false; }}
                    class="w-7 h-7 rounded-full cursor-pointer flex items-center justify-center"
                    style="background: {studyNav.FLAG_COLORS[flagIdx]}; {flagIdx === 0 ? 'border: 2px solid var(--border);' : ''}"
                    title="Flag {flagIdx}"
                  >
                    {#if flagIdx === 0}
                      <svg class="h-3.5 w-3.5" fill="none" stroke="var(--text-muted)" viewBox="0 0 24 24" stroke-width="2">
                        <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
                      </svg>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          </div>

          <!-- Remaining/Reviewed counts -->
          <div class="flex items-center gap-3">
            <span style="font-family: var(--sans); font-size: 12px; color: var(--text-muted);">
              {studyNav.remainingCards} left
            </span>
            <span style="font-family: var(--sans); font-size: 12px; color: var(--accent);">
              {studyNav.reviewedCount} done
            </span>
          </div>

        {:else}
          <!-- Non-study pages -->

          {#if currentPage === 'deckOverview'}
            <!-- Add card button -->
            <button
              onclick={() => navigate('editor')}
              class="neu-subtle neu-btn flex items-center gap-2 px-3.5 py-1.5 rounded-lg cursor-pointer"
              style="background: var(--bg-card);"
            >
              <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--accent);">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
              </svg>
              <span style="font-family: var(--sans); font-size: 13px; color: var(--accent); font-weight: 500;">Add</span>
            </button>
          {/if}

          <!-- Plugins button (always visible outside study) -->
          <button
            onclick={() => showPluginManager = true}
            class="neu-subtle neu-btn flex items-center justify-center w-9 h-9 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Plugins"
            aria-label="Plugins"
          >
            <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
          </button>
        {/if}

        <!-- Settings gear (ALWAYS visible on every page) -->
        <button
          onclick={() => showSettings = true}
          class="neu-subtle neu-btn flex items-center justify-center w-9 h-9 rounded-lg cursor-pointer"
          style="background: var(--bg-card);"
          title="Settings"
          aria-label="Settings"
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
          </svg>
        </button>
      </div>
    </nav>
    
    <!-- Error Banner -->
    {#if collectionStatus === 'error'}
      <div class="bg-red-50 border-b border-danger/20 px-6 py-3">
        <div class="flex items-center gap-2 text-danger text-sm">
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <span>Failed to open collection — {collectionError}</span>
        </div>
      </div>
    {/if}

    <!-- Main Content -->
    <main class="{studyNav.active ? 'flex-1 overflow-hidden' : 'flex-1 overflow-y-auto p-6 lg:p-10'}">
      <!-- Loading State - show skeleton in Dashboard -->
      {#if collectionStatus === 'loading'}
        <div class="max-w-6xl mx-auto">
          <!-- Skeleton Header -->
          <div class="mb-8">
            <div class="skeleton h-8 w-48 mb-2"></div>
            <div class="skeleton h-4 w-64"></div>
          </div>
          <!-- Skeleton Deck Grid -->
          <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {#each Array(3) as _, i}
              <div class="bg-bg-card border border-border rounded-2xl p-6 animate-pulse">
                <div class="skeleton h-6 w-3/4 mb-4"></div>
                <div class="skeleton h-4 w-1/2"></div>
              </div>
            {/each}
          </div>
        </div>
      {:else}
        <!-- Page Router -->
        {#key currentPage}
          {#if currentPage === 'dashboard'}
            <div
              in:fly={fly_if_enabled({ x: -20, duration: 150 })}
            >
              <Dashboard collectionStatus={collectionStatus} onStudy={openDeckOverview} />
            </div>
          {:else if currentPage === 'deckOverview' && activeDeck}
            <div
              in:fly={fly_if_enabled({ x: 20, duration: 150 })}
            >
              <DeckOverview
                deck={activeDeck}
                onStudy={startReview}
                onBrowse={() => navigate('browser')}
                onStats={() => navigate('stats')}
              />
            </div>
          {:else if currentPage === 'study' && currentDeckId}
            <div class="h-full">
              {console.log("Rendering StudyView with deckId:", currentDeckId)}
              <StudyView
                deckId={currentDeckId}
                deckName={currentDeckName}
                onExit={exitReviewMode}
              />
            </div>
          {:else if currentPage === 'editor'}
            <div
              in:fly={fly_if_enabled({ x: 20, duration: 150 })}
              class="max-w-2xl mx-auto"
            >
              <CardEditor
                onBack={() => { navigate(previousPage); editingCard = null; }}
                editCard={editingCard}
              />
            </div>
          {:else if currentPage === 'stats'}
            <div
              in:fly={fly_if_enabled({ x: 20, duration: 150 })}
            >
              <StatsView />
            </div>
          {:else if currentPage === 'browser'}
            <div class="h-full">
              <CardBrowser
                initialQuery={browserQuery}
                onClose={() => { navigate(previousPage); browserQuery = ''; }}
              />
            </div>
          {/if}
        {/key}
      {/if}
    </main>

    <!-- Toast Notifications -->
    <NeuToast />

    <!-- Keyboard Shortcuts Modal -->
    <KeyboardShortcuts isOpen={showKeyboardShortcuts} onClose={() => showKeyboardShortcuts = false} />

    <!-- Import Modal -->
    <ImportModal 
      isOpen={showImportModal} 
      collectionStatus={collectionStatus}
      onClose={() => showImportModal = false}
      onImportComplete={() => {
        // Trigger deck reload in Dashboard
        window.dispatchEvent(new CustomEvent('refresh-decks'));
      }}
    />

    <!-- Notetype Manager Modal -->
    {#if showNotetypeManager}
      <div 
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        onclick={() => showNotetypeManager = false}
        onkeydown={(e) => e.key === 'Escape' && (showNotetypeManager = false)}
        role="dialog"
        aria-modal="true"
        aria-labelledby="notetype-manager-title"
      >
        <div 
          class="bg-bg-card border border-border rounded-2xl shadow-xl w-full max-w-4xl h-[80vh]"
          onclick={(e) => e.stopPropagation()}
          onkeydown={(e) => e.stopPropagation()}
          role="document"
        >
          <NotetypeManager />
        </div>
      </div>
    {/if}

    <!-- Settings Panel -->
    <Settings isOpen={showSettings} onClose={() => showSettings = false} />
    
    <!-- Plugin Manager Modal -->
    <PluginManager isOpen={showPluginManager} onClose={() => showPluginManager = false} />
    
    <!-- Image Occlusion Modal -->
    {#if showImageOcclusion}
      <ImageOcclusion onClose={() => showImageOcclusion = false} />
    {/if}
  </div>
{/if}
