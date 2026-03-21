<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { isTauri } from "@tauri-apps/api/core";
  import Chart from "./Chart.svelte";
  import type { ChartData, ChartOptions } from "chart.js";
  import { addToast } from "./toast";

  // Types
  interface ForecastDay {
    day: number;
    new: number;
    review: number;
    learning: number;
  }

  interface DailyReview {
    date: string;
    count: number;
    time_secs: number;
  }

  interface CardTypeStats {
    new: number;
    learning: number;
    young: number;
    mature: number;
  }

  interface RetentionStats {
    young_retention: number;
    mature_retention: number;
    overall: number;
  }

  interface DailyCount {
    date: string;
    count: number;
  }

  interface ReviewStats {
    forecast: ForecastDay[];
    daily_reviews: DailyReview[];
    hourly_breakdown: number[];
    card_types: CardTypeStats;
    retention: RetentionStats;
    cards_added: DailyCount[];
    current_streak: number;
    longest_streak: number;
    total_reviews: number;
    total_cards: number;
    total_notes: number;
    average_ease: number;
    average_interval_days: number;
  }

  interface DeckInfo {
    id: number;
    name: string;
    card_count: number;
    is_filtered: boolean;
  }

  // State
  let isLoading = $state(true);
  let hasError = $state(false);
  let decks = $state<DeckInfo[]>([]);
  let selectedDeckId = $state<number | null>(null);
  let stats = $state<ReviewStats | null>(null);

  // Load decks and stats on mount
  onMount(async () => {
    const tauriCheck = await isTauri();
    if (!tauriCheck) {
      // Mock data for browser testing
      stats = generateMockStats();
      isLoading = false;
      return;
    }

    try {
      // Load decks
      const allDecks = await invoke<any[]>("get_all_decks");
      decks = allDecks.map((d: any) => ({
        id: d.id,
        name: d.name,
        card_count: d.card_count,
        is_filtered: d.is_filtered
      }));
      
      // Load stats (all decks by default)
      await loadStats(null);
    } catch (error) {
      console.error("Error loading stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load statistics", "error");
      hasError = true;
    } finally {
      isLoading = false;
    }
  });

  async function loadStats(deckId: number | null) {
    isLoading = true;
    try {
      if (deckId === null) {
        stats = await invoke<ReviewStats>("get_review_stats", { deckId: null });
      } else {
        stats = await invoke<ReviewStats>("get_deck_specific_stats", { deckId });
      }
      selectedDeckId = deckId;
    } catch (error) {
      console.error("Error loading stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load statistics", "error");
    } finally {
      isLoading = false;
    }
  }

  function handleDeckChange(event: Event) {
    const target = event.target as HTMLSelectElement;
    const value = target.value;
    const deckId = value === "all" ? null : parseInt(value, 10);
    loadStats(deckId);
  }

  async function handleRetry() {
    hasError = false;
    isLoading = true;
    try {
      const allDecks = await invoke<any[]>("get_all_decks");
      decks = allDecks.map((d: any) => ({
        id: d.id,
        name: d.name,
        card_count: d.card_count,
        is_filtered: d.is_filtered
      }));
      await loadStats(null);
    } catch (error) {
      console.error("Error loading stats:", error);
      addToast(error instanceof Error ? error.message : "Failed to load statistics", "error");
      hasError = true;
    } finally {
      isLoading = false;
    }
  }

  function generateMockStats(): ReviewStats {
    const forecast: ForecastDay[] = [];
    for (let i = 1; i <= 30; i++) {
      forecast.push({
        day: i,
        new: Math.floor(Math.random() * 10),
        review: Math.floor(Math.random() * 30),
        learning: Math.floor(Math.random() * 5),
      });
    }

    const dailyReviews: DailyReview[] = [];
    for (let i = 29; i >= 0; i--) {
      const date = new Date();
      date.setDate(date.getDate() - i);
      dailyReviews.push({
        date: date.toISOString().split('T')[0],
        count: Math.floor(Math.random() * 50) + 10,
        time_secs: Math.floor(Math.random() * 1800),
      });
    }

    const hourlyBreakdown: number[] = [];
    for (let i = 0; i < 24; i++) {
      hourlyBreakdown.push(Math.floor(Math.random() * 20));
    }

    const cardsAdded: DailyCount[] = [];
    for (let i = 29; i >= 0; i--) {
      const date = new Date();
      date.setDate(date.getDate() - i);
      cardsAdded.push({
        date: date.toISOString().split('T')[0],
        count: Math.floor(Math.random() * 15),
      });
    }

    return {
      forecast,
      daily_reviews: dailyReviews,
      hourly_breakdown: hourlyBreakdown,
      card_types: {
        new: 100,
        learning: 15,
        young: 200,
        mature: 500,
      },
      retention: {
        young_retention: 0.85,
        mature_retention: 0.92,
        overall: 0.90,
      },
      cards_added: cardsAdded,
      current_streak: 7,
      longest_streak: 14,
      total_reviews: 5000,
      total_cards: 815,
      total_notes: 600,
      average_ease: 2.65,
      average_interval_days: 45.5,
    };
  }

  function formatTime(secs: number): string {
    const hours = Math.floor(secs / 3600);
    const minutes = Math.floor((secs % 3600) / 60);
    if (hours > 0) {
      return `${hours}h ${minutes}m`;
    }
    return `${minutes} min`;
  }

  function formatPercentage(value: number): string {
    return `${(value * 100).toFixed(1)}%`;
  }

  // Chart: Forecast (stacked bar)
  let forecastChartData = $derived.by(() => {
    if (!stats?.forecast.length) return { labels: [], datasets: [] };

    const labels = stats.forecast.slice(0, 14).map(d => {
      const date = new Date();
      date.setDate(date.getDate() + d.day);
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    });
    
    return {
      labels,
      datasets: [
        {
          label: 'New',
          data: stats.forecast.slice(0, 14).map(d => d.new),
          backgroundColor: 'rgba(96, 165, 250, 0.85)',
          borderRadius: 4,
        },
        {
          label: 'Learning',
          data: stats.forecast.slice(0, 14).map(d => d.learning),
          backgroundColor: 'rgba(251, 191, 36, 0.85)',
          borderRadius: 4,
        },
        {
          label: 'Review',
          data: stats.forecast.slice(0, 14).map(d => d.review),
          backgroundColor: 'rgba(167, 139, 250, 0.85)',
          borderRadius: 4,
        },
      ],
    } as ChartData;
  });

  let forecastChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        stacked: true,
        grid: { display: false },
        ticks: { color: 'var(--text-secondary)', font: { size: 11 } },
      },
      y: {
        stacked: true,
        grid: { color: 'rgba(255,255,255,0.1)' },
        ticks: { color: 'var(--text-secondary)' },
      },
    },
    plugins: {
      legend: {
        position: 'top',
        labels: { color: 'var(--text-secondary)', boxWidth: 12, padding: 10 },
      },
    },
  };

  // Chart: Daily Reviews (bar)
  let dailyReviewsChartData = $derived.by(() => {
    if (!stats?.daily_reviews.length) return { labels: [], datasets: [] };

    const last30 = stats.daily_reviews.slice(-30);
    
    return {
      labels: last30.map(d => {
        const date = new Date(d.date);
        return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
      }),
      datasets: [{
        label: 'Reviews',
        data: last30.map(d => d.count),
        backgroundColor: 'rgba(196, 113, 79, 0.85)',
        borderRadius: 6,
        borderSkipped: false,
      }],
    } as ChartData;
  });

  let dailyReviewsChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        grid: { display: false },
        ticks: { color: 'var(--text-secondary)', font: { size: 10 }, maxRotation: 45 },
      },
      y: {
        grid: { color: 'rgba(255,255,255,0.1)' },
        ticks: { color: 'var(--text-secondary)' },
      },
    },
    plugins: {
      legend: { display: false },
    },
  };

  // Chart: Hourly Breakdown (bar)
  let hourlyChartData = $derived.by(() => {
    if (!stats?.hourly_breakdown) return { labels: [], datasets: [] };

    const hours = Array.from({ length: 24 }, (_, i) => 
      i === 0 ? '12am' : i < 12 ? `${i}am` : i === 12 ? '12pm' : `${i-12}pm`
    );

    return {
      labels: hours,
      datasets: [{
        label: 'Reviews',
        data: stats.hourly_breakdown,
        backgroundColor: 'rgba(52, 211, 153, 0.85)',
        borderRadius: 4,
        borderSkipped: false,
      }],
    } as ChartData;
  });

  let hourlyChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        grid: { display: false },
        ticks: { color: 'var(--text-secondary)', font: { size: 10 } },
      },
      y: {
        grid: { color: 'rgba(255,255,255,0.1)' },
        ticks: { color: 'var(--text-secondary)' },
      },
    },
    plugins: {
      legend: { display: false },
    },
  };

  // Chart: Card Types (doughnut)
  let cardTypesChartData = $derived.by(() => {
    if (!stats) return { labels: [], datasets: [] };

    return {
      labels: ['New', 'Learning', 'Young', 'Mature'],
      datasets: [{
        data: [
          stats.card_types.new,
          stats.card_types.learning,
          stats.card_types.young,
          stats.card_types.mature,
        ],
        backgroundColor: ['#60A5FA', '#FBBF24', '#F97316', '#10B981'],
        borderWidth: 0,
      }],
    } as ChartData;
  });

  let cardTypesChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    plugins: {
      legend: {
        position: 'right',
        labels: { color: 'var(--text-secondary)', boxWidth: 12, padding: 15 },
      },
    },
  } as ChartOptions;

  // Chart: Cards Added (bar)
  let cardsAddedChartData = $derived.by(() => {
    if (!stats?.cards_added.length) return { labels: [], datasets: [] };

    const last30 = stats.cards_added.slice(-30);
    
    return {
      labels: last30.map(d => {
        const date = new Date(d.date);
        return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
      }),
      datasets: [{
        label: 'Cards Added',
        data: last30.map(d => d.count),
        backgroundColor: 'rgba(167, 139, 250, 0.85)',
        borderRadius: 6,
        borderSkipped: false,
      }],
    } as ChartData;
  });

  let cardsAddedChartOptions: ChartOptions = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      x: {
        grid: { display: false },
        ticks: { color: 'var(--text-secondary)', font: { size: 10 }, maxRotation: 45 },
      },
      y: {
        grid: { color: 'rgba(255,255,255,0.1)' },
        ticks: { color: 'var(--text-secondary)' },
      },
    },
    plugins: {
      legend: { display: false },
    },
  };
</script>

<div class="max-w-7xl mx-auto">
  <!-- Header with Deck Filter -->
  <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-4 mb-6">
    <h1 class="text-2xl font-semibold text-text-primary">Statistics</h1>
    
    <!-- Deck Filter (hidden on error) -->
    {#if !hasError}
    <div class="flex items-center gap-3">
      <label for="deck-select" class="text-sm text-text-secondary">Deck:</label>
      <select
        id="deck-select"
        class="px-4 py-2 rounded-lg bg-bg-card border border-border text-text-primary focus:outline-none focus:ring-2 focus:ring-accent"
        value={selectedDeckId === null ? "all" : selectedDeckId.toString()}
        onchange={handleDeckChange}
      >
        <option value="all">All Decks</option>
        {#each decks.filter(d => !d.is_filtered) as deck}
          <option value={deck.id.toString()}>{deck.name}</option>
        {/each}
      </select>
    </div>
    {/if}
  </div>

  <!-- Error State -->
  {#if hasError && !isLoading}
    <div class="bg-bg-card rounded-xl p-8 shadow-warm border border-border text-center mb-8">
      <div class="text-text-secondary mb-4">
        <svg class="w-12 h-12 mx-auto mb-4 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <p class="text-lg">Failed to load statistics</p>
        <p class="text-sm">There was a problem retrieving your study data.</p>
      </div>
      <button
        onclick={handleRetry}
        class="px-6 py-2 bg-accent text-white rounded-lg hover:bg-accent/90 transition-colors"
      >
        Retry
      </button>
    </div>
  {/if}

  <!-- Stats Content (hidden on error) -->
  {#if !hasError}
  <div class="grid grid-cols-2 md:grid-cols-4 lg:grid-cols-6 gap-4 mb-8">
    <!-- Total Cards -->
    <div class="bg-bg-card rounded-xl p-4 shadow-warm border border-border">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-16 h-4 bg-bg-subtle rounded mb-2"></div>
          <div class="w-12 h-8 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="text-sm text-text-secondary mb-1">Total Cards</div>
        <div class="text-2xl font-semibold text-text-primary">{stats?.total_cards ?? 0}</div>
      {/if}
    </div>

    <!-- Total Notes -->
    <div class="bg-bg-card rounded-xl p-4 shadow-warm border border-border">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-16 h-4 bg-bg-subtle rounded mb-2"></div>
          <div class="w-12 h-8 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="text-sm text-text-secondary mb-1">Total Notes</div>
        <div class="text-2xl font-semibold text-text-primary">{stats?.total_notes ?? 0}</div>
      {/if}
    </div>

    <!-- Total Reviews -->
    <div class="bg-bg-card rounded-xl p-4 shadow-warm border border-border">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-16 h-4 bg-bg-subtle rounded mb-2"></div>
          <div class="w-12 h-8 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="text-sm text-text-secondary mb-1">Total Reviews</div>
        <div class="text-2xl font-semibold text-text-primary">{stats?.total_reviews ?? 0}</div>
      {/if}
    </div>

    <!-- Average Ease -->
    <div class="bg-bg-card rounded-xl p-4 shadow-warm border border-border">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-16 h-4 bg-bg-subtle rounded mb-2"></div>
          <div class="w-12 h-8 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="text-sm text-text-secondary mb-1">Avg. Ease</div>
        <div class="text-2xl font-semibold text-text-primary">{stats?.average_ease.toFixed(2) ?? '0.00'}</div>
      {/if}
    </div>

    <!-- Average Interval -->
    <div class="bg-bg-card rounded-xl p-4 shadow-warm border border-border">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-20 h-4 bg-bg-subtle rounded mb-2"></div>
          <div class="w-16 h-8 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="text-sm text-text-secondary mb-1">Avg. Interval</div>
        <div class="text-2xl font-semibold text-text-primary">{stats?.average_interval_days.toFixed(1) ?? '0'}d</div>
      {/if}
    </div>

    <!-- Retention -->
    <div class="bg-bg-card rounded-xl p-4 shadow-warm border border-border">
      {#if isLoading}
        <div class="animate-pulse">
          <div class="w-16 h-4 bg-bg-subtle rounded mb-2"></div>
          <div class="w-12 h-8 bg-bg-subtle rounded"></div>
        </div>
      {:else}
        <div class="text-sm text-text-secondary mb-1">Retention</div>
        <div class="text-2xl font-semibold text-text-primary">{formatPercentage(stats?.retention.overall ?? 0)}</div>
      {/if}
    </div>
  </div>

  <!-- Streak Section -->
  <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border mb-6">
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-4">
        <div class="p-3 bg-accent/10 rounded-xl">
          <svg class="w-8 h-8 text-accent" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 18.657A8 8 0 016.343 7.343S7 9 9 10c0-2 .5-5 2.986-7C14 5 16.09 5.777 17.656 7.343A7.975 7.975 0 0120 13a7.975 7.975 0 01-2.343 5.657z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.879 16.121A3 3 0 1012.015 11L11 14H9c0 .768.293 1.536.879 2.121z" />
          </svg>
        </div>
        <div>
          <div class="text-sm text-text-secondary">Current Streak</div>
          <div class="text-3xl font-semibold text-text-primary">
            {stats?.current_streak ?? 0} <span class="text-lg font-normal text-text-secondary">days</span>
          </div>
        </div>
      </div>
      <div class="text-right">
        <div class="text-sm text-text-secondary">Longest Streak</div>
        <div class="text-2xl font-semibold text-text-primary">
          {stats?.longest_streak ?? 0} <span class="text-sm font-normal text-text-secondary">days</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Charts Grid -->
  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <!-- Review Forecast -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border">
      <h3 class="text-lg font-semibold text-text-primary mb-4">Review Forecast</h3>
      {#if isLoading}
        <div class="skeleton h-64 rounded-lg"></div>
      {:else}
        <div class="h-64">
          <Chart type="bar" data={forecastChartData} options={forecastChartOptions} />
        </div>
      {/if}
    </div>

    <!-- Daily Reviews -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border">
      <h3 class="text-lg font-semibold text-text-primary mb-4">Reviews per Day (Last 30 Days)</h3>
      {#if isLoading}
        <div class="skeleton h-64 rounded-lg"></div>
      {:else}
        <div class="h-64">
          <Chart type="bar" data={dailyReviewsChartData} options={dailyReviewsChartOptions} />
        </div>
      {/if}
    </div>

    <!-- Hourly Breakdown -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border">
      <h3 class="text-lg font-semibold text-text-primary mb-4">Study Time (Hourly)</h3>
      {#if isLoading}
        <div class="skeleton h-64 rounded-lg"></div>
      {:else}
        <div class="h-64">
          <Chart type="bar" data={hourlyChartData} options={hourlyChartOptions} />
        </div>
      {/if}
    </div>

    <!-- Card Types -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border">
      <h3 class="text-lg font-semibold text-text-primary mb-4">Card Types</h3>
      {#if isLoading}
        <div class="skeleton h-64 rounded-lg"></div>
      {:else}
        <div class="h-64">
          <Chart type="doughnut" data={cardTypesChartData} options={cardTypesChartOptions} />
        </div>
        
        <!-- Card Type Legend -->
        <div class="grid grid-cols-2 gap-4 mt-4">
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-sm bg-[#60A5FA]"></div>
            <span class="text-sm text-text-secondary">New: {stats?.card_types.new ?? 0}</span>
          </div>
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-sm bg-[#FBBF24]"></div>
            <span class="text-sm text-text-secondary">Learning: {stats?.card_types.learning ?? 0}</span>
          </div>
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-sm bg-[#F97316]"></div>
            <span class="text-sm text-text-secondary">Young: {stats?.card_types.young ?? 0}</span>
          </div>
          <div class="flex items-center gap-2">
            <div class="w-3 h-3 rounded-sm bg-[#10B981]"></div>
            <span class="text-sm text-text-secondary">Mature: {stats?.card_types.mature ?? 0}</span>
          </div>
        </div>
      {/if}
    </div>

    <!-- Cards Added -->
    <div class="bg-bg-card rounded-2xl p-6 shadow-warm border border-border lg:col-span-2">
      <h3 class="text-lg font-semibold text-text-primary mb-4">Cards Added (Last 30 Days)</h3>
      {#if isLoading}
        <div class="skeleton h-48 rounded-lg"></div>
      {:else}
        <div class="h-48">
          <Chart type="bar" data={cardsAddedChartData} options={cardsAddedChartOptions} />
        </div>
      {/if}
    </div>
  </div>

  <!-- Retention Stats -->
  <div class="mt-6 bg-bg-card rounded-2xl p-6 shadow-warm border border-border">
    <h3 class="text-lg font-semibold text-text-primary mb-4">Retention Rates</h3>
    {#if isLoading}
      <div class="flex gap-8">
        <div class="skeleton w-24 h-16 rounded"></div>
        <div class="skeleton w-24 h-16 rounded"></div>
        <div class="skeleton w-24 h-16 rounded"></div>
      </div>
    {:else}
      <div class="flex flex-wrap gap-8">
        <div class="text-center">
          <div class="text-3xl font-semibold text-text-primary">{formatPercentage(stats?.retention.young_retention ?? 0)}</div>
          <div class="text-sm text-text-secondary">Young Card Retention</div>
        </div>
        <div class="text-center">
          <div class="text-3xl font-semibold text-text-primary">{formatPercentage(stats?.retention.mature_retention ?? 0)}</div>
          <div class="text-sm text-text-secondary">Mature Card Retention</div>
        </div>
        <div class="text-center">
          <div class="text-3xl font-semibold text-accent">{formatPercentage(stats?.retention.overall ?? 0)}</div>
          <div class="text-sm text-text-secondary">Overall Retention</div>
        </div>
      </div>
    {/if}
  </div>
  {/if}
</div>

<style>
  /* Custom scrollbar for the container */
  :global(.overflow-auto) {
    scrollbar-width: thin;
    scrollbar-color: var(--bg-subtle) transparent;
  }

  :global(.overflow-auto::-webkit-scrollbar) {
    width: 8px;
    height: 8px;
  }

  :global(.overflow-auto::-webkit-scrollbar-track) {
    background: transparent;
  }

  :global(.overflow-auto::-webkit-scrollbar-thumb) {
    background-color: var(--bg-subtle);
    border-radius: 4px;
  }
</style>
