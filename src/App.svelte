<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, onDestroy } from "svelte";
  import { isTauri } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { fade, fly } from "svelte/transition";
  import StudyView from "./lib/StudyView.svelte";
  import CardEditor from "./lib/CardEditor.svelte";
  import Dashboard from "./lib/Dashboard.svelte";
  import StatsView from "./lib/StatsView.svelte";
  import Toast from "./lib/Toast.svelte";
  import KeyboardShortcuts from "./lib/KeyboardShortcuts.svelte";
  import ImportModal from "./lib/ImportModal.svelte";
  import { exportCollectionColpkg, ImportError } from "./lib/importer";
  import { prefs } from "./lib/prefs";
  import { addToast } from "./lib/toast";
  import { fly_if_enabled } from "./lib/animate";
  import { initMathJax } from "./lib/mathjax";
  import CardBrowser from "./lib/CardBrowser.svelte";
  import NotetypeManager from "./lib/NotetypeManager.svelte";

  // Page state
  type Page = 'dashboard' | 'study' | 'editor' | 'stats' | 'browser';
  let currentPage: Page = $state('dashboard');
  let browserQuery = $state('');

  // Function to open browser with a specific query (e.g., from leech toast)
  function openBrowserWithQuery(query: string) {
    browserQuery = query;
    currentPage = 'browser';
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
  
  // Notetype manager state
  let showNotetypeManager = $state(false);

  async function handleExportCollection() {
    try {
      await exportCollectionColpkg();
      addToast("Collection exported successfully", "success");
    } catch (e) {
      if (e instanceof ImportError && e.isCancelled) return;
      addToast(e instanceof Error ? e.message : "Export failed", "error");
    }
  }

  // Deck stats for dashboard
  let deckStats: Array<{ id: number; name: string; new_cards: number; learn_cards: number; review_cards: number }> = $state([]);
  let isCollectionOpen = $state(false);

  // Study view state
  let currentDeckId: number | null = $state(null);
  let currentDeckName = $state("");

  let unlistenCollectionReady: UnlistenFn | null = null;

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
    
    // Listen for collection ready event
    unlistenCollectionReady = await listen<{ success: boolean; error?: string }>("collection:ready", (event) => {
      if (event.payload.success) {
        collectionStatus = 'ready';
        isCollectionOpen = true;
        getDeckStats();
      } else {
        collectionStatus = 'error';
        collectionError = event.payload.error || 'Unknown error';
      }
    });
    
    // Fire and forget - don't await
    // Load prefs, init MathJax, and init collection concurrently
    Promise.all([prefs.load(), initMathJax(), invoke("init_standalone_collection")]).catch((error) => {
      collectionStatus = 'error';
      collectionError = error instanceof Error ? error.message : String(error);
    });
  });

  onDestroy(() => {
    if (unlistenCollectionReady) {
      unlistenCollectionReady();
    }
  });

  async function getDeckStats() {
    if (!isCollectionOpen) return;
    try {
      const result = await invoke<Array<{ id: number; name: string; new_cards: number; learn_cards: number; review_cards: number }>>("get_deck_stats");
      deckStats = result;
    } catch (error) {
      console.error("Error loading deck stats:", error);
      deckStats = [];
    }
  }

  function startReview(deckId: number, deckName: string) {
    currentDeckId = deckId;
    currentDeckName = deckName;
    currentPage = 'study';
  }

  function exitReviewMode() {
    currentPage = 'dashboard';
    currentDeckId = null;
    currentDeckName = "";
    getDeckStats();
  }

  // Global keyboard shortcuts
  function handleGlobalKeydown(event: KeyboardEvent) {
    if (event.key === "?") {
      event.preventDefault();
      showKeyboardShortcuts = !showKeyboardShortcuts;
    }
  }
</script>

<svelte:window on:keydown={handleGlobalKeydown} />

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
    <nav class="h-12 bg-bg-base border-b border-border flex items-center px-6 justify-between">
      <div class="text-sm font-medium text-text-primary">Anki Wrapper</div>
      <div class="flex gap-4">
        <button
          onclick={() => currentPage = 'dashboard'}
          class="p-2 rounded-lg transition-colors cursor-pointer {currentPage === 'dashboard' ? 'text-accent bg-accent-soft' : 'text-text-secondary hover:bg-bg-subtle'}"
          aria-label="Dashboard"
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
          </svg>
        </button>
        <button
          onclick={() => currentPage = 'editor'}
          class="p-2 rounded-lg transition-colors cursor-pointer {currentPage === 'editor' ? 'text-accent bg-accent-soft' : 'text-text-secondary hover:bg-bg-subtle'}"
          aria-label="Add Card"
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
          </svg>
        </button>
        <button
          onclick={() => currentPage = 'browser'}
          class="p-2 rounded-lg transition-colors cursor-pointer {currentPage === 'browser' ? 'text-accent bg-accent-soft' : 'text-text-secondary hover:bg-bg-subtle'}"
          aria-label="Browse cards"
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
          </svg>
        </button>
        <button
          onclick={() => currentPage = 'stats'}
          class="p-2 rounded-lg transition-colors cursor-pointer {currentPage === 'stats' ? 'text-accent bg-accent-soft' : 'text-text-secondary hover:bg-bg-subtle'}"
          aria-label="Stats"
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
        </button>
        <button
          onclick={() => showImportModal = true}
          class="px-3 py-1.5 bg-bg-subtle text-text-primary rounded-xl hover:bg-border transition-colors text-sm font-medium flex items-center gap-2 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
          disabled={collectionStatus !== 'ready'}
          title={collectionStatus === 'loading' ? 'Collection initializing...' : collectionStatus === 'error' ? 'Collection failed to load' : 'Import deck'}
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
          </svg>
          Import Deck
        </button>
        <button
          onclick={handleExportCollection}
          class="p-2 rounded-lg transition-colors cursor-pointer text-text-secondary hover:bg-bg-subtle disabled:opacity-50 disabled:cursor-not-allowed"
          disabled={collectionStatus !== 'ready'}
          title={collectionStatus === 'loading' ? 'Collection initializing...' : collectionStatus === 'error' ? 'Collection failed to load' : 'Export collection'}
          aria-label="Export collection"
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
          </svg>
        </button>
        <button
          onclick={() => showSettings = true}
          class="p-2 rounded-lg transition-colors cursor-pointer text-text-secondary hover:bg-bg-subtle"
          title="Settings"
          aria-label="Settings"
        >
          <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
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
    <main class="flex-1 overflow-y-auto p-6">
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
        {#if currentPage === 'dashboard'}
          <div
            in:fly={fly_if_enabled({ x: -30 })}
            out:fly={fly_if_enabled({ x: 30 })}
          >
            <Dashboard collectionStatus={collectionStatus} />
          </div>
        {:else if currentPage === 'study' && currentDeckId}
          <div
            in:fly={fly_if_enabled({ x: 30 })}
            out:fly={fly_if_enabled({ x: -30 })}
            class="h-full"
          >
            <StudyView 
              deckId={currentDeckId} 
              deckName={currentDeckName}
              onExit={exitReviewMode}
            />
          </div>
        {:else if currentPage === 'editor'}
          <div
            in:fly={fly_if_enabled({ x: 30 })}
            out:fly={fly_if_enabled({ x: -30 })}
            class="max-w-2xl mx-auto"
          >
            <CardEditor onBack={() => currentPage = 'dashboard'} />
          </div>
        {:else if currentPage === 'stats'}
          <div
            in:fly={fly_if_enabled({ x: 30 })}
            out:fly={fly_if_enabled({ x: -30 })}
          >
            <StatsView />
          </div>
        {:else if currentPage === 'browser'}
          <div class="h-full">
            <CardBrowser 
              initialQuery={browserQuery} 
              onClose={() => { currentPage = 'dashboard'; browserQuery = ''; }} 
            />
          </div>
        {/if}
      {/if}
    </main>

    <!-- Toast Notifications -->
    <Toast />

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
    {#if showSettings}
      <div 
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        onclick={() => showSettings = false}
        onkeydown={(e) => e.key === 'Escape' && (showSettings = false)}
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
      >
        <div 
          class="bg-bg-card border border-border rounded-2xl shadow-xl w-full max-w-md p-6"
          onclick={(e) => e.stopPropagation()}
          onkeydown={(e) => e.stopPropagation()}
          role="document"
        >
          <div class="flex items-center justify-between mb-6">
            <h2 id="settings-title" class="text-lg font-semibold text-text-primary">Settings</h2>
            <button 
              onclick={() => showSettings = false}
              class="p-1 rounded-lg text-text-secondary hover:text-text-primary hover:bg-bg-subtle transition-colors"
              aria-label="Close settings"
            >
              <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div class="space-y-6">
            <!-- Note Types Section -->
            <div>
              <h3 class="text-sm font-medium text-text-secondary mb-3">Note Types</h3>
              <button
                onclick={() => { showSettings = false; showNotetypeManager = true; }}
                class="w-full px-4 py-2 bg-bg-subtle hover:bg-bg-active text-text-primary rounded-lg transition-colors text-left"
              >
                Manage Note Types
              </button>
            </div>

            <!-- Animations Section -->
            <div>
              <h3 class="text-sm font-medium text-text-secondary mb-3">Animations</h3>
              <div class="space-y-3">
                <label class="flex items-center justify-between cursor-pointer">
                  <span class="text-text-primary">Enable animations</span>
                  <input 
                    type="checkbox" 
                    checked={prefs.animations_enabled}
                    onchange={(e) => {
                      prefs.animations_enabled = e.currentTarget.checked;
                      prefs.save();
                    }}
                    class="w-5 h-5 rounded border-border text-accent focus:ring-accent focus:ring-offset-0 bg-bg-base"
                  />
                </label>
                <label class="flex items-center justify-between cursor-pointer">
                  <span class="text-text-primary">Reduce motion</span>
                  <input 
                    type="checkbox" 
                    checked={prefs.reduce_motion}
                    onchange={(e) => {
                      prefs.reduce_motion = e.currentTarget.checked;
                      prefs.save();
                    }}
                    class="w-5 h-5 rounded border-border text-accent focus:ring-accent focus:ring-offset-0 bg-bg-base"
                  />
                </label>
              </div>
            </div>
          </div>
        </div>
      </div>
    {/if}
  </div>
{/if}
