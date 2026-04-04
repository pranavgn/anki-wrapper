<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { addToast } from "./toast";
  import { getChartColors } from "./chartTheme";
  import CalendarWidget from "./widgets/CalendarWidget.svelte";
  import DeckSettings from "./DeckSettings.svelte";
  import DeckOptions from "./DeckOptions.svelte";

  interface DeckStat {
    id: number;
    name: string;
    short_name: string;
    level: number;
    new_count: number;
    learn_count: number;
    review_count: number;
    card_count: number;
    is_filtered: boolean;
  }

  let {
    deck,
    onStudy,
    onBrowse,
    onStats,
    onDeckRenamed = (newName: string) => {},
    onDeckDeleted = () => {},
  }: {
    deck: DeckStat;
    onStudy: (deckId: number, deckName: string) => void;
    onBrowse: () => void;
    onStats: () => void;
    onDeckRenamed?: (newName: string) => void;
    onDeckDeleted?: () => void;
  } = $props();

  let showDeckSettings = $state(false);
  let showDeckOptions = $state(false);
  let renamedName = $state<string | null>(null);
  const displayName = $derived(renamedName ?? deck.short_name ?? deck.name);

  // Real data state
  let weekData = $state<number[]>([]);
  let streak = $state(0);
  let isLoading = $state(true);

  const weekDays = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

  // Card type distribution using theme colors
  const colors = getChartColors();
  const cardTypes = $derived([
    { name: 'New', count: deck.new_count, color: colors.newCards },
    { name: 'Learning', count: deck.learn_count, color: colors.learning },
    { name: 'Review', count: deck.review_count, color: colors.accent }
  ]);

  const totalCards = $derived(deck.card_count);

  // Calculate progress percentage
  const progressPercent = $derived(
    totalCards > 0 
      ? Math.round(((deck.new_count + deck.learn_count + deck.review_count) / totalCards) * 100) 
      : 0
  );

  // Fetch real data on mount
  onMount(async () => {
    try {
      const [heatmap, stats] = await Promise.all([
        invoke<Array<{date: string; count: number}>>("get_review_heatmap", { days: 7, deckId: deck.id }),
        invoke<any>("get_review_stats", { deckId: deck.id })
      ]);
      
      weekData = heatmap.map(d => d.count);
      streak = stats.current_streak;
    } catch (e) {
      console.error("Failed to load deck overview data:", e);
      weekData = [];
      streak = 0;
    } finally {
      isLoading = false;
    }
  });

  function handleStudy() {
    onStudy(deck.id, deck.name);
  }

  function handleBrowse() {
    onBrowse();
  }

  function handleStats() {
    onStats();
  }

  async function handleRebuild() {
    try {
      const count = await invoke<number>('rebuild_filtered_deck', { deckId: deck.id });
      addToast(`Rebuilt filtered deck: ${count} cards`, 'success');
    } catch (error) {
      addToast(error instanceof Error ? error.message : 'Failed to rebuild filtered deck', 'error');
    }
  }

  async function handleEmpty() {
    try {
      await invoke('empty_filtered_deck', { deckId: deck.id });
      addToast('Filtered deck emptied', 'success');
    } catch (error) {
      addToast(error instanceof Error ? error.message : 'Failed to empty filtered deck', 'error');
    }
  }
</script>

<div class="max-w-[600px] mx-auto px-6 py-12" style="animation: fadeUp 0.4s ease-out;">
  <!-- Header -->
  <div class="text-center mb-10">
    <div class="text-6xl mb-4" style="animation: float 3s ease-in-out infinite;">📚</div>
    <h1 style="font-family: var(--serif); font-size: 32px; font-weight: 600; color: var(--text-primary); margin-bottom: 8px;">
      {displayName}
    </h1>
    <p style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
      {totalCards} cards · {streak} day streak 🔥
    </p>
    <button
      onclick={() => showDeckSettings = true}
      class="neu-subtle neu-btn"
      style="
        margin-top: 12px;
        padding: 8px 16px;
        border-radius: var(--radius-md);
        display: inline-flex;
        align-items: center;
        gap: 6px;
        cursor: pointer;
      "
    >
      <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
      <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">Deck Settings</span>
    </button>
  </div>

  <!-- Stat Pills -->
  <div class="flex justify-center gap-4 mb-10">
    <div
      class="neu-raised text-center"
      style="
        border-radius: var(--radius-md);
        padding: 16px 24px;
      "
    >
      <div style="font-family: var(--serif); font-size: 28px; font-weight: 700; color: {cardTypes[0].color};">
        {deck.new_count}
      </div>
      <div style="font-family: var(--sans); font-size: 11px; text-transform: uppercase; color: var(--text-muted); letter-spacing: 0.05em;">
        New
      </div>
    </div>
    <div
      class="neu-raised text-center"
      style="
        border-radius: var(--radius-md);
        padding: 16px 24px;
      "
    >
      <div style="font-family: var(--serif); font-size: 28px; font-weight: 700; color: {cardTypes[1].color};">
        {deck.learn_count}
      </div>
      <div style="font-family: var(--sans); font-size: 11px; text-transform: uppercase; color: var(--text-muted); letter-spacing: 0.05em;">
        Learning
      </div>
    </div>
    <div
      class="neu-raised text-center"
      style="
        border-radius: var(--radius-md);
        padding: 16px 24px;
      "
    >
      <div style="font-family: var(--serif); font-size: 28px; font-weight: 700; color: {cardTypes[2].color};">
        {deck.review_count}
      </div>
      <div style="font-family: var(--sans); font-size: 11px; text-transform: uppercase; color: var(--text-muted); letter-spacing: 0.05em;">
        Review
      </div>
    </div>
  </div>

  <!-- Progress Bar -->
  <div class="mb-10">
    <div 
      class="neu-pressed"
      style="
        background: var(--bg-deep);
        box-shadow: var(--neu-down);
        border-radius: 10px;
        height: 7px;
        overflow: hidden;
      "
    >
      <div 
        style="
          height: 100%;
          width: {progressPercent}%;
          background: linear-gradient(90deg, var(--success), var(--success) 80%, transparent);
          border-radius: 10px;
          transition: width 0.6s ease-out;
        "
      ></div>
    </div>
    <p class="text-center mt-2" style="font-family: var(--sans); font-size: 12px; color: var(--text-muted);">
      {progressPercent}% complete
    </p>
  </div>

  <!-- Mini Charts Section -->
  <div class="grid grid-cols-2 gap-4 mb-10">
    <!-- This Week Chart -->
    <div 
      class="neu-raised"
      style="
        background: var(--bg-card);
        box-shadow: var(--neu-up);
        border-radius: var(--radius-md);
        padding: 20px;
      "
    >
      <h3 style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary); margin-bottom: 16px;">
        This Week
      </h3>
      {#if weekData.length === 0 || weekData.every(v => v === 0)}
        <div style="display: flex; align-items: center; justify-content: center; height: 96px;">
          <p style="font-family: var(--sans); font-size: 12px; color: var(--text-muted);">No recent reviews</p>
        </div>
      {:else}
        <svg viewBox="0 0 200 100" class="w-full h-24">
          {#each weekData as value, i}
            <rect
              x={i * 28 + 10}
              y={100 - (value / 20) * 80}
              width="20"
              height={(value / 20) * 80}
              fill="var(--accent)"
              rx="3"
              opacity="0.8"
            />
            <text
              x={i * 28 + 20}
              y="95"
              text-anchor="middle"
              fill="var(--text-muted)"
              font-size="8"
              font-family="var(--sans)"
            >
              {weekDays[i]}
            </text>
          {/each}
        </svg>
      {/if}
    </div>

    <!-- Study Calendar Widget -->
    <CalendarWidget />
  </div>

  <!-- Card Types Doughnut -->
  <div 
    class="neu-raised mb-10"
    style="
      background: var(--bg-card);
      box-shadow: var(--neu-up);
      border-radius: var(--radius-md);
      padding: 24px;
    "
  >
    <h3 style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary); margin-bottom: 20px;">
      Card Types
    </h3>
    <div class="flex items-center gap-6">
      <svg viewBox="0 0 100 100" class="w-24 h-24">
        <!-- Background circle -->
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke="var(--bg-deep)"
          stroke-width="12"
        />
        <!-- New cards segment -->
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke={cardTypes[0].color}
          stroke-width="12"
          stroke-dasharray="{(deck.new_count / totalCards) * 251.2} 251.2"
          stroke-dashoffset="0"
          transform="rotate(-90 50 50)"
        />
        <!-- Learning cards segment -->
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke={cardTypes[1].color}
          stroke-width="12"
          stroke-dasharray="{(deck.learn_count / totalCards) * 251.2} 251.2"
          stroke-dashoffset="-{(deck.new_count / totalCards) * 251.2}"
          transform="rotate(-90 50 50)"
        />
        <!-- Review cards segment -->
        <circle
          cx="50"
          cy="50"
          r="40"
          fill="none"
          stroke={cardTypes[2].color}
          stroke-width="12"
          stroke-dasharray="{(deck.review_count / totalCards) * 251.2} 251.2"
          stroke-dashoffset="-{((deck.new_count + deck.learn_count) / totalCards) * 251.2}"
          transform="rotate(-90 50 50)"
        />
      </svg>
      <div class="flex-1 grid grid-cols-1 gap-3">
        {#each cardTypes as type}
          <div class="flex items-center gap-3">
            <div 
              class="w-3 h-3 rounded-full"
              style="background: {type.color};"
            ></div>
            <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">
              {type.name}
            </span>
            <span 
              class="ml-auto"
              style="font-family: var(--sans); font-size: 13px; font-weight: 600; color: var(--text-primary);"
            >
              {type.count}
            </span>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <!-- Action Buttons -->
  <div class="space-y-3">
    <button
      onclick={handleStudy}
      class="w-full neu-raised neu-btn cursor-pointer"
      style="
        background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 85%, #000));
        border-radius: var(--radius-md);
        padding: 18px 24px;
        border: none;
      "
    >
      <span style="font-family: var(--serif); font-size: 22px; font-weight: 600; color: white;">
        Study Now
      </span>
    </button>

    {#if deck.is_filtered}
      <div class="flex gap-2 mt-2">
        <button
          onclick={handleRebuild}
          class="flex-1 neu-subtle neu-btn cursor-pointer flex items-center justify-center gap-2"
          style="border-radius: var(--radius-md); padding: 12px 16px;"
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
          </svg>
          <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">Rebuild</span>
        </button>
        <button
          onclick={handleEmpty}
          class="flex-1 neu-subtle neu-btn cursor-pointer flex items-center justify-center gap-2"
          style="border-radius: var(--radius-md); padding: 12px 16px;"
        >
          <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
          </svg>
          <span style="font-family: var(--sans); font-size: 13px; color: var(--text-secondary);">Empty</span>
        </button>
      </div>
    {/if}

    <button
      onclick={handleBrowse}
      class="w-full neu-subtle neu-btn flex items-center justify-center gap-2 cursor-pointer"
      style="
        border-radius: var(--radius-md);
        padding: 14px 24px;
      "
    >
      <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 10h18M3 14h18m-9-4v8m-7 0h14a2 2 0 002-2V8a2 2 0 00-2-2H5a2 2 0 00-2 2v8a2 2 0 002 2z" />
      </svg>
      <span style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
        Browse All Cards
      </span>
    </button>

    <button
      onclick={handleStats}
      class="w-full neu-subtle neu-btn flex items-center justify-center gap-2 cursor-pointer"
      style="
        border-radius: var(--radius-md);
        padding: 14px 24px;
      "
    >
      <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" style="color: var(--text-secondary);">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
      <span style="font-family: var(--sans); font-size: 14px; color: var(--text-secondary);">
        View Statistics
      </span>
    </button>
  </div>
  <!-- Deck Settings Modal -->
  <DeckSettings
    deckId={deck.id}
    deckName={displayName}
    isFiltered={deck.is_filtered}
    isOpen={showDeckSettings}
    onClose={() => showDeckSettings = false}
    onRenamed={(newName) => {
      renamedName = newName;
      onDeckRenamed(newName);
    }}
    onDeleted={onDeckDeleted}
    onOpenOptions={() => showDeckOptions = true}
  />

  <!-- Deck Options Modal -->
  <DeckOptions
    deckId={deck.id}
    deckName={displayName}
    isFiltered={deck.is_filtered}
    isOpen={showDeckOptions}
    onClose={() => showDeckOptions = false}
  />
</div>

<style>
  @keyframes float {
    0%, 100% { transform: translateY(0); }
    50% { transform: translateY(-4px); }
  }
</style>
