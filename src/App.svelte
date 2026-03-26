<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount, tick } from "svelte";
  import { fly } from "svelte/transition";
  import Dashboard from "./lib/Dashboard.svelte";
  import DeckOverview from "./lib/DeckOverview.svelte";
  import NeuToast from "./lib/ui/NeuToast.svelte";
  import { exportCollectionColpkg, ImportError } from "./lib/importer";
  import { prefs } from "./lib/prefs.svelte.ts";
  import { addToast } from "./lib/toast";
  import { fly_if_enabled } from "./lib/animate.svelte.ts";
  import { pluginEngine } from "./lib/pluginEngine";
  import { loadAllPlugins } from "./lib/pluginLoader";
  import { studyNav } from "./lib/studyNav.svelte.ts";
  import {
    initNotifications,
    startDailyReminderCheck,
    startScheduleChecker,
    stopAll,
    sendMilestoneNotification,
    sendStudyReminder,
  } from "./lib/notifications";
  import "./lib/statsAPI";
  import { loadCustomTheme } from "./lib/customTheme";
  import { initMathJax } from "./lib/mathjax";
  import { loader } from "./lib/loadingTracker.svelte";

  // Lazy-loaded components — only fetched when first needed
  const LazyStatsView     = () => import("./lib/StatsView.svelte");
  const LazyCardBrowser   = () => import("./lib/CardBrowser.svelte");
  const LazyNotetypeManager = () => import("./lib/NotetypeManager.svelte");
  const LazySettings      = () => import("./lib/Settings.svelte");
  const LazyImageOcclusion = () => import("./lib/ImageOcclusion.svelte");
  const LazyPluginManager = () => import("./lib/PluginManager.svelte");

  // Resolved component references (populated on first use)
  let StatsViewComponent: any = $state(null);
  let CardBrowserComponent: any = $state(null);
  let NotetypeManagerComponent: any = $state(null);
  let SettingsComponent: any = $state(null);
  let ImageOcclusionComponent: any = $state(null);
  let PluginManagerComponent: any = $state(null);

  // Helper: resolve a lazy component
  async function loadComponent(
    loader: () => Promise<any>,
    setter: (mod: any) => void
  ) {
    const mod = await loader();
    setter(mod.default);
  }

  // Load StatsView when navigating to stats page
  $effect(() => {
    if (currentPage === 'stats' && !StatsViewComponent) {
      loadComponent(LazyStatsView, (c) => StatsViewComponent = c);
    }
  });

  // Load CardBrowser when navigating to browser page
  $effect(() => {
    if (currentPage === 'browser' && !CardBrowserComponent) {
      loadComponent(LazyCardBrowser, (c) => CardBrowserComponent = c);
    }
  });

  // Load modals when opened
  $effect(() => {
    if (showNotetypeManager && !NotetypeManagerComponent) {
      loadComponent(LazyNotetypeManager, (c) => NotetypeManagerComponent = c);
    }
  });
  $effect(() => {
    if (showSettings && !SettingsComponent) {
      loadComponent(LazySettings, (c) => SettingsComponent = c);
    }
  });
  $effect(() => {
    if (showImageOcclusion && !ImageOcclusionComponent) {
      loadComponent(LazyImageOcclusion, (c) => ImageOcclusionComponent = c);
    }
  });
  $effect(() => {
    if (showPluginManager && !PluginManagerComponent) {
      loadComponent(LazyPluginManager, (c) => PluginManagerComponent = c);
    }
  });

  // Page state
  type Page = 'dashboard' | 'deckOverview' | 'study' | 'editor' | 'stats' | 'browser';
  let currentPage: Page = $state('dashboard');
  let previousPage: Page = $state('dashboard');
  let browserQuery = $state('');
  
  // Focus management
  let mainContentRef: HTMLElement | null = $state(null);
  
  // Page order for determining animation direction
  const pageOrder: Page[] = ['dashboard', 'deckOverview', 'study', 'editor', 'stats', 'browser'];
  
  // Track nav animation direction
  let navDirection: 'forward' | 'back' = $state('forward');
  
  // Card editor state
  let editingCard = $state<any>(null);
  
  // Handle edit card from detail panel
  function handleEditCard(card: any) {
    editingCard = card;
    navigate('editor');
  }

  // Navigation function
  async function navigate(page: Page) {
    const prevIdx = pageOrder.indexOf(currentPage);
    const nextIdx = pageOrder.indexOf(page);
    navDirection = nextIdx >= prevIdx ? 'forward' : 'back';
    previousPage = currentPage;
    currentPage = page;
    
    // Focus main content after navigation
    await tick();
    if (mainContentRef) {
      mainContentRef.focus();
    }
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
  let isCollectionOpen: boolean = $state(false);

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

  // Study view state
  let currentDeckId: number | null = $state(null);
  let currentDeckName = $state("");
  let activeDeck: any = $state(null);

  // Get due card count for notifications
  async function getDueCardCount(): Promise<number> {
    const decks = await invoke<Array<{id: number; name: string; new_count: number; learn_count: number; review_count: number}>>("get_all_decks");
    return decks.reduce((sum: number, d: any) => sum + (d.new_count || 0) + (d.learn_count || 0) + (d.review_count || 0), 0);
  }

  // Get deck stats for dashboard
  async function getDeckStats() {
    try {
      await invoke("get_all_decks");
    } catch (e) {
      console.error("Failed to get deck stats:", e);
    }
  }

  // Initialize on mount
  onMount(async () => {
    // Expose function to open browser with query
    (window as any).openCardBrowser = (query: string) => {
      openBrowserWithQuery(query);
    };

    // Platform detection for traffic light spacing (sync, instant)
    const nav = navigator.platform.toLowerCase();
    if (nav.includes('mac')) {
      document.documentElement.classList.add('platform-macos');
    } else if (nav.includes('win')) {
      document.documentElement.classList.add('platform-windows');
    } else {
      document.documentElement.classList.add('platform-linux');
    }

    // ── Tauri check (fast, synchronous-ish) ──
    // Use sync detection: __TAURI_INTERNALS__ exists immediately
    if (!(window as any).__TAURI_INTERNALS__) {
      isRunningInBrowser = true;
      browserCheckComplete = true;
      collectionStatus = 'error';
      collectionError = 'Tauri desktop environment required';
      return;
    }
    browserCheckComplete = true;

    // ── Register loading steps ──
    loader.register([
      { name: 'Preferences',  weight: 1 },
      { name: 'MathJax',      weight: 1 },
      { name: 'Collection',   weight: 3 }, // heaviest
      { name: 'Plugins',      weight: 1 },
    ]);
    console.log(`[Loader] Registered steps. Initial progress: ${loader.progress}%`);

    // ── Step 1: Preferences ──
    loader.start('Preferences');
    console.log(`[Loader] Starting Preferences. Progress: ${loader.progress}%`);
    try { await prefs.load(); } catch(e) { console.error('Prefs error:', e); }
    loader.finish('Preferences');
    console.log(`[Loader] Finished Preferences. Progress: ${loader.progress}%`);

    // ── Step 2: MathJax ──
    loader.start('MathJax');
    console.log(`[Loader] Starting MathJax. Progress: ${loader.progress}%`);
    try { await initMathJax(); } catch(e) { console.error('MathJax error:', e); }
    loader.finish('MathJax');
    console.log(`[Loader] Finished MathJax. Progress: ${loader.progress}%`);

    // ── Step 3: Collection (critical path) ──
    loader.start('Collection');
    console.log(`[Loader] Starting Collection. Progress: ${loader.progress}%`);
    try {
      await invoke("init_standalone_collection");
      collectionStatus = 'ready';
      isCollectionOpen = true;
    } catch (error) {
      collectionStatus = 'error';
      collectionError = error instanceof Error ? error.message : String(error);
      loader.fail('Collection', collectionError);
      console.log(`[Loader] Failed Collection. Progress: ${loader.progress}%`);
      return; // don't continue to plugins
    }
    loader.finish('Collection');
    console.log(`[Loader] Finished Collection. Progress: ${loader.progress}%`);

    // ── Step 4: Plugins (deferred, non-blocking) ──
    if (collectionStatus === 'ready') {
      getDeckStats();

      // Use requestIdleCallback to load plugins without blocking the UI
      const loadPlugins = async () => {
        loader.start('Plugins');
        console.log(`[Loader] Starting Plugins. Progress: ${loader.progress}%`);
        try {
          await loadAllPlugins();
          await pluginEngine.runAction('app:ready', {});
        } catch(e) { console.error('Plugin error:', e); }
        loader.finish('Plugins');
        console.log(`[Loader] Finished Plugins. Progress: ${loader.progress}%`);
      };

      if ('requestIdleCallback' in window) {
        requestIdleCallback(() => loadPlugins());
      } else {
        setTimeout(() => loadPlugins(), 0);
      }
    }
  });

  function startReview(deckId: number, deckName: string) {
    console.debug("startReview called with deckId:", deckId, "deckName:", deckName);
    currentDeckId = deckId;
    currentDeckName = deckName;
    navigate('study');
    console.debug("State after startReview: currentPage=", currentPage, ", currentDeckId=", currentDeckId);
  }

  function openDeckOverview(deckId: number, deckName: string) {
    activeDeck = { id: deckId, name: deckName };
    currentDeckId = deckId;
    currentDeckName = deckName;
    navigate('deckOverview');
  }

  async function exitReviewMode() {
    if (currentPage === 'study' && previousPage === 'deckOverview') {
      navDirection = 'back';
      navigate('deckOverview');
    } else {
      navDirection = 'back';
      navigate('dashboard');
      currentDeckId = null;
      currentDeckName = "";
      activeDeck = null;
    }

    // Check for streak milestones when exiting study mode
    if (prefs.notifications_enabled) {
      try {
        const stats = await invoke<any>("get_review_stats", { deckId: null });
        if (stats.current_streak > 0 && stats.current_streak % 7 === 0) {
          sendMilestoneNotification('streak', stats.current_streak);
        }
      } catch (e) { /* ignore */ }
    }
  }

  function goToDashboard() {
    navDirection = 'back';
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

<svelte:window onkeydown={handleGlobalKeydown} onclick={() => { if (studyNav.showFlagPicker) studyNav.showFlagPicker = false; }} />

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
  <div class="h-screen bg-bg-base flex flex-col overflow-hidden">
    <!-- Skip to main content link for keyboard users -->
    <a href="#main-content" class="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-bg-card focus:text-text-primary focus:rounded-lg focus:shadow-warm">
      Skip to main content
    </a>
    
    <!-- Top Navigation -->
    <nav data-tauri-drag-region class="app-navbar flex items-center px-6 py-3" style="background: var(--bg-card); border-bottom: 1px solid var(--border); position: relative; z-index: 30; flex-shrink: 0;" aria-label="Main navigation">

      <!-- Traffic light gutter (macOS only — the class is added in onMount) -->
      <div class="traffic-light-gutter"></div>

      {#key currentPage}
      <!-- ═══ LEFT SECTION ═══ -->
      <div class="flex items-center gap-2 {prefs.reduce_motion ? '' : 'nav-anim-left'}" style="position: relative; z-index: 2; flex-shrink: 0;">
        {#if currentPage === 'dashboard'}
          <!-- Logo -->
          <svg class="h-7 w-7" viewBox="0 0 100 100" fill="none" xmlns="http://www.w3.org/2000/svg" style="flex-shrink: 0; margin-left: 8px;">
            <rect width="100" height="100" rx="22" fill="var(--accent)" />
            <path d="M30 70V32l20 24 20-24v38" stroke="white" stroke-width="7" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
          </svg>
          <span style="font-family: var(--serif); font-size: 22px; font-weight: 600; color: var(--text-primary); letter-spacing: -0.02em;">Mnemora</span>

        {:else if studyNav.active}
          <!-- Study mode: End + Undo -->
          <button
            type="button"
            onclick={() => studyNav.exit?.()}
            class="neu-subtle neu-btn flex items-center gap-2 px-3 py-1.5 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 19l-7-7m0 0l7-7m-7 7h18" />
            </svg>
            <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">End</span>
          </button>
          {#if studyNav.canUndo}
            <button
              type="button"
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
          <!-- All other pages: Back -->
          <button
            type="button"
            onclick={goToDashboard}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Back to dashboard"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
            </svg>
          </button>
        {/if}
      </div>

      <!-- ═══ CENTER SECTION ═══ -->
      <div class="{prefs.reduce_motion ? '' : (navDirection === 'forward' ? 'nav-anim-center' : 'nav-anim-center-reverse')}" style="position: absolute; left: 0; right: 0; top: 0; bottom: 0; display: flex; align-items: center; justify-content: center; gap: 8px; pointer-events: none; z-index: 1;">
        <div style="pointer-events: auto; display: flex; align-items: center; gap: 8px;">
          {#if currentPage === 'deckOverview' && activeDeck}
            <!-- Breadcrumb: Decks > DeckName -->
            <button
              type="button"
              onclick={goToDashboard}
              style="font-family: var(--sans); font-size: 13px; color: var(--text-muted); cursor: pointer; background: none; border: none; padding: 0;"
            >
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
            <span style="font-family: var(--sans); font-size: 14px; color: var(--text-primary); font-weight: 500;">
              Statistics
            </span>

          {:else if currentPage === 'browser'}
            <span style="font-family: var(--sans); font-size: 14px; color: var(--text-primary); font-weight: 500;">
              Card Browser
            </span>
          {/if}
        </div>
      </div>

      <!-- ═══ RIGHT SECTION ═══ -->
      <div class="flex items-center gap-1.5 {prefs.reduce_motion ? '' : 'nav-anim-right'}" style="margin-left: auto; position: relative; z-index: 2; flex-shrink: 0;">
        {#if studyNav.active}
          <!-- Study: minimal — just flag + settings -->
          <div class="relative">
            <button
              type="button"
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
                    type="button"
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
          <div class="nav-divider"></div>
          <button
            type="button"
            onclick={() => showSettings = true}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Settings"
            aria-label="Settings"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>

        {:else if currentPage === 'dashboard'}
          <!-- Dashboard: full toolbar -->
          <button
            type="button"
            onclick={() => navigate('editor')}
            class="neu-subtle neu-btn flex items-center gap-2 px-3.5 py-1.5 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--accent);">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            <span style="font-family: var(--sans); font-size: 13px; color: var(--accent); font-weight: 500;">Add</span>
          </button>
          <div class="nav-divider"></div>
          <button
            type="button"
            onclick={() => showImportModal = true}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Import"
            aria-label="Import"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
            </svg>
          </button>
          <button
            type="button"
            onclick={handleExportCollection}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Export"
            aria-label="Export"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
            </svg>
          </button>
          <div class="nav-divider"></div>
          <button
            type="button"
            onclick={() => showPluginManager = true}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Plugins"
            aria-label="Plugins"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
          </button>
          <button
            type="button"
            onclick={() => showSettings = true}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Settings"
            aria-label="Settings"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>

        {:else if currentPage === 'deckOverview'}
          <!-- Deck overview: Add + utilities -->
          <button
            type="button"
            onclick={() => navigate('editor')}
            class="neu-subtle neu-btn flex items-center gap-2 px-3.5 py-1.5 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--accent);">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
            </svg>
            <span style="font-family: var(--sans); font-size: 13px; color: var(--accent); font-weight: 500;">Add</span>
          </button>
          <div class="nav-divider"></div>
          <button
            type="button"
            onclick={() => showPluginManager = true}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Plugins"
            aria-label="Plugins"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
          </button>
          <button
            type="button"
            onclick={() => showSettings = true}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Settings"
            aria-label="Settings"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>

        {:else}
          <!-- Editor, Stats, Browser: just utilities -->
          <button
            type="button"
            onclick={() => showPluginManager = true}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Plugins"
            aria-label="Plugins"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
          </button>
          <button
            type="button"
            onclick={() => showSettings = true}
            class="neu-subtle neu-btn flex items-center justify-center w-8 h-8 rounded-lg cursor-pointer"
            style="background: var(--bg-card);"
            title="Settings"
            aria-label="Settings"
          >
            <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);" aria-hidden="true">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          </button>
        {/if}
      </div>
      {/key}
    </nav>
    
    <!-- Error Banner -->
    {#if collectionStatus === 'error'}
      <div role="alert" class="bg-red-50 border-b border-danger/20 px-6 py-3">
        <div class="flex items-center gap-2 text-danger text-sm">
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
          </svg>
          <span>Failed to open collection — {collectionError}</span>
        </div>
      </div>
    {/if}

    <!-- Loading State -->
    {#if collectionStatus === 'loading'}
      <div class="flex-1 flex items-center justify-center">
        <div class="w-48 flex flex-col items-center gap-3">
          <div
            class="w-full h-1 rounded-full overflow-hidden"
            style="background: var(--bg-subtle);"
            role="progressbar"
            aria-label="Loading: {loader.progress}%"
            aria-valuenow={loader.progress}
            aria-valuemin={0}
            aria-valuemax={100}
          >
            <div
              class="h-full rounded-full"
              style="
                background: var(--accent);
                width: {loader.progress}%;
                transition: width 0.25s ease-out;
              "
            ></div>
          </div>
          <span
            style="font-family: var(--sans); font-size: 11px; color: var(--text-tertiary, var(--text-secondary)); letter-spacing: 0.02em;"
          >
            {loader.progress}%
          </span>
        </div>
      </div>
    {:else}
    <!-- Main Content -->
    <main id="main-content" bind:this={mainContentRef} tabindex="-1" class="{studyNav.active ? 'flex-1 min-h-0 overflow-hidden flex flex-col' : 'flex-1 min-h-0 overflow-y-auto p-6 lg:p-10 flex flex-col'}">
      <!-- Page Router -->
      {#key currentPage}
        {#if currentPage === 'dashboard'}
          <div
            in:fly={fly_if_enabled({ x: -20, duration: 150 })}
          >
            <Dashboard onStudy={openDeckOverview} />
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
              {#await import('./lib/StudyView.svelte') then mod}
                <mod.default
                  deckId={currentDeckId}
                  deckName={currentDeckName}
                  onExit={exitReviewMode}
                />
              {/await}
            </div>
          {:else if currentPage === 'editor'}
            <div
              in:fly={fly_if_enabled({ x: 20, duration: 150 })}
              class="max-w-2xl mx-auto"
            >
              {#await import('./lib/CardEditor.svelte') then mod}
                <mod.default
                  onBack={() => { navigate(previousPage); editingCard = null; }}
                  editCard={editingCard}
                />
              {/await}
            </div>
          {:else if currentPage === 'stats'}
            <div
              in:fly={fly_if_enabled({ x: 20, duration: 150 })}
            >
              {#if StatsViewComponent}
                <StatsViewComponent />
              {:else}
                <div class="flex items-center justify-center p-12">
                  <div class="animate-spin rounded-full h-6 w-6 border-2" style="border-color: var(--accent); border-top-color: transparent;"></div>
                </div>
              {/if}
            </div>
          {:else if currentPage === 'browser'}
            <div class="h-full">
              {#if CardBrowserComponent}
                <CardBrowserComponent
                  initialQuery={browserQuery}
                  onClose={() => { currentPage = 'dashboard'; browserQuery = ''; }}
                />
              {:else}
                <div class="flex items-center justify-center p-12">
                  <div class="animate-spin rounded-full h-6 w-6 border-2" style="border-color: var(--accent); border-top-color: transparent;"></div>
                </div>
              {/if}
            </div>
          {/if}
        {/key}
    </main>

    <!-- Toast Notifications -->
    <NeuToast />

    <!-- Keyboard Shortcuts Modal -->
    {#if showKeyboardShortcuts}
      {#await import('./lib/KeyboardShortcuts.svelte') then mod}
        <mod.default isOpen={showKeyboardShortcuts} onClose={() => showKeyboardShortcuts = false} />
      {/await}
    {/if}

    <!-- Import Modal -->
    {#if showImportModal}
      {#await import('./lib/ImportModal.svelte') then mod}
        <mod.default
          isOpen={showImportModal}
          collectionStatus={collectionStatus}
          onClose={() => showImportModal = false}
          onImportComplete={() => {
            // Trigger deck reload in Dashboard
            window.dispatchEvent(new CustomEvent('refresh-decks'));
          }}
        />
      {/await}
    {/if}

    <!-- Notetype Manager Modal -->
    {#if showNotetypeManager}
      <div
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
        onclick={() => showNotetypeManager = false}
        onkeydown={(e) => e.key === 'Escape' && (showNotetypeManager = false)}
        role="dialog"
        aria-modal="true"
        aria-labelledby="notetype-manager-title"
        tabindex="-1"
      >
        <div
          class="bg-bg-card border border-border rounded-2xl shadow-xl w-full max-w-4xl h-[80vh]"
          role="document"
        >
          {#if NotetypeManagerComponent}
            <NotetypeManagerComponent />
          {:else}
            <div class="flex items-center justify-center h-full">
              <div class="animate-spin rounded-full h-6 w-6 border-2" style="border-color: var(--accent); border-top-color: transparent;"></div>
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Settings Panel -->
    {#if showSettings}
      {#if SettingsComponent}
        <SettingsComponent onClose={() => showSettings = false} />
      {/if}
    {/if}
    
    <!-- Plugin Manager Modal -->
    {#if showPluginManager}
      {#if PluginManagerComponent}
        <PluginManagerComponent onClose={() => showPluginManager = false} />
      {/if}
    {/if}
    
    <!-- Image Occlusion Modal -->
    {#if showImageOcclusion}
      {#if ImageOcclusionComponent}
        <ImageOcclusionComponent onClose={() => showImageOcclusion = false} />
      {/if}
    {/if}
    {/if}
  </div>
{/if}

